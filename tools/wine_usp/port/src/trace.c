/* trace.c — optional per-lookup OpenType shaping trace for the standalone
 * Uniscribe port (port extension, LGPL).
 *
 * When armed via usp_trace_begin(), the shaping driver records a bounded
 * sequence of glyph-run snapshots:
 *     "cmap"                        initial cmap glyphs
 *     "<feat>#<k>/<n>"              after GSUB lookup k (1-based) of feature
 *                                   <feat> (4-char tag) with n lookups total
 *     "final"                       exact glyph run returned by ScriptShapeOpenType
 *
 * Fixed-size global buffers: tracing is best-effort and single-threaded
 * (a dedicated trace session must not overlap another shaping call).
 *
 * The host (pyusp engine) loads wineusp.dll, calls usp_trace_begin() before
 * ScriptShapeOpenType and usp_trace_stop() after, then reads stages with
 * usp_trace_count()/usp_trace_stage(). */
#include <windows.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

#define USPTRACE_MAX_STAGES 1024
#define USPTRACE_MAX_GLYPHS 768
#define USPTRACE_NAME_LEN   40

/* stderr echo (port debug): set env USP_TR=1 to print every snapshot */
static int usp_tr_env(void)
{
    static int c = -1;
    if (c < 0) c = (getenv("USP_TR") != NULL) ? 1 : 0;
    return c;
}

typedef struct {
    char name[USPTRACE_NAME_LEN];
    int  glyph_count;
    WORD glyphs[USPTRACE_MAX_GLYPHS];
} UspTraceStage;

static UspTraceStage g_stages[USPTRACE_MAX_STAGES];
static volatile LONG g_count = 0;
static volatile LONG g_active = 0;

int usp_trace_begin(void)
{
    g_count = 0;
    g_active = 1;
    return 0;
}

void usp_trace_stop(void)
{
    g_active = 0;
}

int usp_trace_count(void)
{
    return (int)g_count;
}

void usp_trace_snapshot(const char *name, const WORD *glyphs, int glyph_count)
{
    UspTraceStage *s;
    int n = (int)g_count;
    int cap = (int)sizeof(g_stages[0].glyphs) / (int)sizeof(g_stages[0].glyphs[0]);

    if (usp_tr_env() && name && glyphs && glyph_count > 0)
    {
        int i, show = glyph_count > 40 ? 40 : glyph_count;
        fprintf(stderr, "[usp] %s:", name);
        for (i = 0; i < show; i++)
            fprintf(stderr, " %u", glyphs[i]);
        fprintf(stderr, "\n");
    }

    if (!g_active) return;
    if (n < 0 || n >= USPTRACE_MAX_STAGES) return;
    if (!name || !glyphs || glyph_count <= 0) return;
    if (glyph_count > cap) glyph_count = cap;

    s = &g_stages[n];
    memset(s->name, 0, sizeof(s->name));
    if (name) strncpy(s->name, name, sizeof(s->name) - 1);
    s->glyph_count = glyph_count;
    memcpy(s->glyphs, glyphs, (size_t)glyph_count * sizeof(WORD));
    InterlockedIncrement(&g_count);
}

/* Record the state after a whole-run application of lookup `lookup_index`
 * (0-based) of feature `feature` (4-char tag, may be unterminated). */
void usp_trace_lookup(const char *feature, int lookup_index, int lookup_count,
                      const WORD *glyphs, int glyph_count)
{
    char name[USPTRACE_NAME_LEN];
    char tag[5];

    if (!g_active) return;
    if (!feature || !glyphs || glyph_count <= 0) return;

    tag[0] = tag[1] = tag[2] = tag[3] = ' ';
    tag[4] = 0;
    if (feature[0]) tag[0] = feature[0];
    if (feature[1]) tag[1] = feature[1];
    if (feature[2]) tag[2] = feature[2];
    if (feature[3]) tag[3] = feature[3];

    _snprintf(name, sizeof(name), "%s#%d/%d", tag, lookup_index + 1, lookup_count);
    name[sizeof(name) - 1] = 0;
    usp_trace_snapshot(name, glyphs, glyph_count);
}

/* Copy stage `i` out. name_out: caller buffer (>=name_len, 0 for none).
 * Returns 0 on success, -1 on out-of-range. */
int usp_trace_stage(int i, char *name_out, int name_len,
                    int *glyph_count_out, const WORD **glyphs_out)
{
    const UspTraceStage *s;

    if (i < 0 || i >= (int)g_count) return -1;
    s = &g_stages[i];
    if (name_out && name_len > 0)
    {
        strncpy(name_out, s->name, (size_t)name_len - 1);
        name_out[name_len - 1] = 0;
    }
    if (glyph_count_out) *glyph_count_out = s->glyph_count;
    if (glyphs_out) *glyphs_out = s->glyphs;
    return 0;
}
