# -*- coding: utf-8 -*-
"""Apply LookupFlag mark-filtering (IgnoreMarks etc.) to the Wine GSUB chain
matching in port/src/opentype.c. Each replacement asserts an expected count."""
import io
import sys

P = r"D:\Github\pyusp\tools\wine_usp\port\src\opentype.c"
src = io.open(P, encoding="utf-8").read()


def rep(old, new, count=1, label=""):
    global src
    n = src.count(old)
    if n != count:
        print(f"FAIL [{label}] expected {count} got {n}")
        sys.exit(1)
    src = src.replace(old, new)
    print(f"ok [{label}] x{n}")


# 1) helper functions (insert before GSUB_apply_SingleSubst)
HELPERS = """
/* ---- LookupFlag mark filtering (port) ----
 * GSUB/GPOS lookup flags (0x2 IgnoreBaseGlyphs, 0x4 IgnoreLigatures,
 * 0x8 IgnoreMarks) make those glyph classes transparent during context
 * matching. Wine ignored LookupFlag; Mongolian 'rclt' lookups rely on it
 * (e.g. lookup62: IgnoreMarks -> the FVS marker glyph is skipped when
 * matching the lookahead vowel). */
static const void *gdef_class_table(const void *gdef)
{
    WORD off;
    if (!gdef) return NULL;
    off = GET_BE_WORD(((const GDEF_Header *)gdef)->GlyphClassDef);
    return off ? (const BYTE *)gdef + off : NULL;
}

static int glyph_ignored(const void *gdef, WORD flag, WORD glyph)
{
    WORD cls;
    if (!gdef) return 0;
    cls = OT_get_glyph_class(gdef_class_table(gdef), glyph);
    if ((flag & 0x0002) && cls == BaseGlyph) return 1;
    if ((flag & 0x0004) && cls == LigatureGlyph) return 1;
    if ((flag & 0x0008) && cls == MarkGlyph) return 1;
    /* MarkAttachmentType (0xFF00) and UseMarkFilteringSet (0x10) not handled */
    return 0;
}

/* buffer position of the n-th (0-based) non-ignored glyph starting at
 * `start` stepping `dir`; -1 when none. n=0 is the first non-ignored at/after
 * start. */
static int nth_nonignored(const void *gdef, WORD flag, const WORD *glyphs,
                          INT count, INT start, INT dir, INT n)
{
    INT pos = start, seen = 0;
    if (n < 0) return -1;
    while (pos >= 0 && pos < count)
    {
        if (!glyph_ignored(gdef, flag, glyphs[pos]))
        {
            if (seen == n) return pos;
            seen++;
        }
        pos += dir;
    }
    return -1;
}

"""
rep('static INT GSUB_apply_SingleSubst(const OT_LookupTable *look,',
    HELPERS + 'static INT GSUB_apply_SingleSubst(const OT_LookupTable *look,',
    1, "helpers")

# 2) GSUB_apply_lookup forward decl + definition: add gdef param
OLD = "static INT GSUB_apply_lookup(const OT_LookupList* lookup, INT lookup_index, WORD *glyphs, INT glyph_index, INT write_dir, INT *glyph_count)"
NEW = "static INT GSUB_apply_lookup(const OT_LookupList* lookup, INT lookup_index, const void *gdef, WORD *glyphs, INT glyph_index, INT write_dir, INT *glyph_count)"
rep(OLD, NEW, 2, "apply_lookup decl+def")

# 3) recursive calls inside Context/Chain
rep("newIndex = GSUB_apply_lookup(lookup, lookup_index, glyphs, g, write_dir, glyph_count);",
    "newIndex = GSUB_apply_lookup(lookup, lookup_index, gdef, glyphs, g, write_dir, glyph_count);",
    2, "recursive newIndex")
rep("new_index = GSUB_apply_lookup(lookup, lookup_index, glyphs, g, write_dir, glyph_count);",
    "new_index = GSUB_apply_lookup(lookup, lookup_index, gdef, glyphs, g, write_dir, glyph_count);",
    3, "recursive new_index")
rep("return GSUB_apply_lookup(lookup, lookup_index, glyphs, glyph_index, write_dir, glyph_count);",
    "return GSUB_apply_lookup(lookup, lookup_index, gdef, glyphs, glyph_index, write_dir, glyph_count);",
    1, "OpenType call")

# 4) function signatures: ContextSubst, ChainContextSubst, ReverseChain
rep("static INT GSUB_apply_ContextSubst(const OT_LookupList* lookup, const OT_LookupTable *look, WORD *glyphs, INT glyph_index, INT write_dir, INT *glyph_count)",
    "static INT GSUB_apply_ContextSubst(const OT_LookupList* lookup, const OT_LookupTable *look, const void *gdef, WORD *glyphs, INT glyph_index, INT write_dir, INT *glyph_count)",
    1, "ContextSubst sig")
rep("static INT GSUB_apply_ChainContextSubst(const OT_LookupList* lookup, INT lookup_index, const OT_LookupTable *look, WORD *glyphs, INT glyph_index, INT write_dir, INT *glyph_count)",
    "static INT GSUB_apply_ChainContextSubst(const OT_LookupList* lookup, INT lookup_index, const OT_LookupTable *look, const void *gdef, WORD *glyphs, INT glyph_index, INT write_dir, INT *glyph_count)",
    1, "ChainContext sig")
rep("static INT GSUB_apply_ReverseChainSingleSubst(const OT_LookupTable *look, WORD *glyphs, INT glyph_index, INT write_dir, INT *glyph_count)",
    "static INT GSUB_apply_ReverseChainSingleSubst(const OT_LookupTable *look, const void *gdef, WORD *glyphs, INT glyph_index, INT write_dir, INT *glyph_count)",
    1, "Reverse sig")

# 5) dispatch in GSUB_apply_lookup body
rep("return GSUB_apply_ContextSubst(lookup, look, glyphs, glyph_index, write_dir, glyph_count);",
    "return GSUB_apply_ContextSubst(lookup, look, gdef, glyphs, glyph_index, write_dir, glyph_count);",
    1, "dispatch context")
rep("return GSUB_apply_ChainContextSubst(lookup, lookup_index, look, glyphs, glyph_index, write_dir, glyph_count);",
    "return GSUB_apply_ChainContextSubst(lookup, lookup_index, look, gdef, glyphs, glyph_index, write_dir, glyph_count);",
    1, "dispatch chain")
rep("return GSUB_apply_ReverseChainSingleSubst(look, glyphs, glyph_index, write_dir, glyph_count);",
    "return GSUB_apply_ReverseChainSingleSubst(look, gdef, glyphs, glyph_index, write_dir, glyph_count);",
    1, "dispatch reverse")

# 6) ChainContextSubst: local flag var
rep("static INT GSUB_apply_ChainContextSubst(const OT_LookupList* lookup, INT lookup_index, const OT_LookupTable *look, const void *gdef, WORD *glyphs, INT glyph_index, INT write_dir, INT *glyph_count)\n{\n    int j;\n\n    TRACE(\"Chaining Contextual Substitution Subtable\\n\");",
    "static INT GSUB_apply_ChainContextSubst(const OT_LookupList* lookup, INT lookup_index, const OT_LookupTable *look, const void *gdef, WORD *glyphs, INT glyph_index, INT write_dir, INT *glyph_count)\n{\n    int j;\n    WORD flag = GET_BE_WORD(look->LookupFlag);\n\n    TRACE(\"Chaining Contextual Substitution Subtable\\n\");",
    1, "chain flag var")

# 7) format-3 matching: skip ignored glyphs in backtrack & lookahead
rep("""                offset = GET_BE_WORD(backtrack->Coverage[k]);
                if (GSUB_is_glyph_covered((const BYTE *)ccsf1 + offset,
                        glyphs[glyph_index + (dirBacktrack * (k + 1))]) == -1)
                    break;""",
    """                offset = GET_BE_WORD(backtrack->Coverage[k]);
                {
                    int bp = nth_nonignored(gdef, flag, glyphs, *glyph_count, glyph_index, dirBacktrack, k + 1);
                    if (bp < 0 || GSUB_is_glyph_covered((const BYTE *)ccsf1 + offset, glyphs[bp]) == -1)
                        break;
                }""",
    1, "fmt3 backtrack mark-skip")
rep("""                offset = GET_BE_WORD(lookahead->Coverage[k]);
                if (GSUB_is_glyph_covered((const BYTE *)ccsf1 + offset,
                        glyphs[glyph_index + (dirLookahead * (input_count + k))]) == -1)
                    break;""",
    """                offset = GET_BE_WORD(lookahead->Coverage[k]);
                {
                    int lp = nth_nonignored(gdef, flag, glyphs, *glyph_count, glyph_index, dirLookahead, input_count + k);
                    if (lp < 0 || GSUB_is_glyph_covered((const BYTE *)ccsf1 + offset, glyphs[lp]) == -1)
                        break;
                }""",
    1, "fmt3 lookahead mark-skip")

# 8) OpenType_apply_GSUB_lookup definition signature + call to apply_lookup already done
rep("int OpenType_apply_GSUB_lookup(const void *table, unsigned int lookup_index, WORD *glyphs,\n        unsigned int glyph_index, int write_dir, int *glyph_count)",
    "int OpenType_apply_GSUB_lookup(const void *table, const void *gdef, unsigned int lookup_index, WORD *glyphs,\n        unsigned int glyph_index, int write_dir, int *glyph_count)",
    1, "OpenType sig")

io.open(P, "w", encoding="utf-8", newline="").write(src)
print("ALL PATCHES APPLIED")
