// pyusp._pyusp — native PyO3 (abi3) binding over the pyusp engine.
//
// Windows-only: the engine drives usp10.dll (Uniscribe) which we load
// dynamically (bundled copy preferred via the Python wrapper).
//
// The Python wrapper in `pyusp/__init__.py` passes the bundled usp10.dll
// path explicitly; callers get the babelsoft `/api/opentype/shape` dict via
// `pyusp.shape_with_uniscribe(...)`.

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

/// Shape `text` with the font bytes `data` and return the babelsoft engine
/// JSON string (same schema as the `pyusp` CLI stdout).
#[pyfunction]
#[pyo3(
    signature = (data, text, *, script = "", language = "", direction = "auto",
                features = None, usp10 = None, trace = false)
)]
fn shape_json(
    py: Python<'_>,
    data: &[u8],
    text: &str,
    script: &str,
    language: &str,
    direction: &str,
    features: Option<String>,
    usp10: Option<String>,
    trace: bool,
) -> PyResult<String> {
    let tmp = temp_font_path();
    std::fs::write(&tmp, data).map_err(|e| PyRuntimeError::new_err(format!("write font: {e}")))?;

    let opts = pyusp::ShapeOpts {
        font: tmp.to_string_lossy().into_owned(),
        text: text.to_owned(),
        script: script.to_owned(),
        language: language.to_owned(),
        direction: direction.to_owned(),
        show_all: false,
        usp10_path: usp10,
        features_arg: features.unwrap_or_default(),
        trace,
    };

    let result = py.allow_threads(|| {
        let r = catch_unwind(AssertUnwindSafe(|| pyusp::shape_json(opts)));
        let _ = std::fs::remove_file(&tmp);
        r
    });

    match result {
        Ok(Ok(json)) => Ok(json),
        Ok(Err(e)) => Err(PyRuntimeError::new_err(e)),
        Err(_) => Err(PyRuntimeError::new_err(
            "pyusp engine panicked (unsupported usp10 build? see README)",
        )),
    }
}

fn temp_font_path() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("pyusp_{}_{}.font", std::process::id(), n))
}

#[pymodule]
fn _pyusp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(shape_json, m)?)?;
    Ok(())
}
