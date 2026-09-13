// pyusp — thin CLI over the Uniscribe shaping-trace engine.
//
// Usage:
//   pyusp --font <path> --text <str> [--script <iso>] [--language <bcp47>]
//         [--direction auto|ltr|rtl] [--features <list>] [--usp10 <path>]
//         [--out <file>]
//
// --features takes a HarfBuzz-style comma-separated list — "kern", "+kern",
// "-kern", "kern=0", "kern=on|off", "aalt=2" — mapping onto Uniscribe's
// OPENTYPE_FEATURE_RECORD lParameter (0 = off, 1 = on/first alternate, >1 =
// alternate index). An explicit value overrides a +/- prefix ("+kern=0" is
// off), and malformed items are errors rather than being dropped. Features
// need a backend that implements feature records (system usp10, or
// --textshaping); the Wine port logs "Ranges not supported yet" and is
// refused instead of silently ignoring them.

use std::process::ExitCode;

fn arg_val(args: &[String], key: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == key {
            return it.next().cloned();
        }
    }
    None
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "pyusp — Uniscribe (usp10) OpenType shaping tracer\n\
             usage: pyusp --font <path> --text <str> [--script <iso>]\n\
                    [--language <bcp47>] [--direction auto|ltr|rtl]\n\
                    [--features <list>] [--usp10 <dll>] [--out <file>]\n\
             --features   : HarfBuzz-style list, e.g. kern,+kern,-kern,kern=0,\n\
                            aalt=2 (explicit =value overrides +/-; errors are\n\
                            fatal). Needs system usp10 or --textshaping — the\n\
                            Wine port cannot apply feature records. Note the\n\
                            list *replaces* the script's default GSUB features\n\
                            for the run rather than adding to them.\n\
             --wine-bytes : drive wineusp via raw font bytes (NULL hdc),\n\
                             the cross-platform path (no GDI font required)"
        );
        return ExitCode::SUCCESS;
    }
    // RE helper: --find-sig <hex> prints every RVA where the signature occurs
    // in the loaded module's executable sections (--scan-module <dll>, default
    // usp10.dll). No shaping. Windows-only (scans a loaded PE module).
    #[cfg(windows)]
    if let Some(hex) = arg_val(&args, "--find-sig") {
        let module = arg_val(&args, "--scan-module").or_else(|| arg_val(&args, "--usp10"));
        match pyusp::scan_module_for_sig(module.as_deref(), &hex) {
            Ok(rvas) => {
                for r in &rvas {
                    println!("{r:#010x}");
                }
                println!("found {} match(es)", rvas.len());
            }
            Err(e) => {
                eprintln!("scan error: {e}");
                return ExitCode::FAILURE;
            }
        }
        return ExitCode::SUCCESS;
    }

    let font = match arg_val(&args, "--font") {
        Some(f) => f,
        None => {
            eprintln!("--font required (or use --find-sig for the RE helper)");
            return ExitCode::FAILURE;
        }
    };
    let text = match arg_val(&args, "--text") {
        Some(t) => t,
        None => {
            eprintln!("--text required");
            return ExitCode::FAILURE;
        }
    };
    let opts = pyusp::ShapeOpts {
        font,
        text,
        script: arg_val(&args, "--script").unwrap_or_default(),
        language: arg_val(&args, "--language").unwrap_or_default(),
        direction: arg_val(&args, "--direction").unwrap_or_else(|| "auto".into()),
        show_all: args.iter().any(|a| a == "--show-all-lookups"),
        usp10_path: arg_val(&args, "--usp10"),
        features_arg: arg_val(&args, "--features").unwrap_or_default(),
        trace: args.iter().any(|a| a == "--trace"),
        wine_bytes: args.iter().any(|a| a == "--wine-bytes"),
        textshaping: args.iter().any(|a| a == "--textshaping"),
    };
    match pyusp::shape_json(opts) {
        Ok(json) => {
            match arg_val(&args, "--out") {
                Some(path) => {
                    if let Err(e) = std::fs::write(&path, &json) {
                        eprintln!("write {}: {e}", path);
                        return ExitCode::FAILURE;
                    }
                }
                None => println!("{json}"),
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("pyusp error: {e}");
            ExitCode::FAILURE
        }
    }
}
