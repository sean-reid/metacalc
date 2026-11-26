; The reader's public routines as the draw page calls them: the draw page includes this file for
; the stubs and again, with FAR_ALIASES defined, around the code that calls the reader, so that
; its `call lib_glyphs` reaches far_lib_glyphs, and with FAR_UNALIASES after it. The entry page
; includes it for the stubs. A name here is listed three times, once in each section.
#ifdef FAR_STUBS
FAR_DATA_ENTRY(lib_read, far_lib_read)
FAR_DATA_ENTRY(lib_open, far_lib_open)
FAR_DATA_ENTRY(lib_assertion_count, far_lib_assertion_count)
FAR_DATA_ENTRY(lib_symbol_counts, far_lib_symbol_counts)
FAR_DATA_ENTRY(lib_symbol_name, far_lib_symbol_name)
FAR_DATA_ENTRY(lib_label, far_lib_label)
FAR_DATA_ENTRY(lib_record, far_lib_record)
FAR_DATA_ENTRY(lib_conclusion, far_lib_conclusion)
FAR_DATA_ENTRY(lib_hypothesis, far_lib_hypothesis)
FAR_DATA_ENTRY(lib_sections, far_lib_sections)
FAR_DATA_ENTRY(lib_title_count, far_lib_title_count)
FAR_DATA_ENTRY(lib_title, far_lib_title)
FAR_DATA_ENTRY(lib_title_for, far_lib_title_for)
FAR_DATA_ENTRY(lib_glyph_map, far_lib_glyph_map)
FAR_DATA_ENTRY(lib_glyphs, far_lib_glyphs)
FAR_DATA_ENTRY(lib_glyph, far_lib_glyph)
FAR_DATA_ENTRY(lib_syntax_count, far_lib_syntax_count)
FAR_DATA_ENTRY(lib_syntax, far_lib_syntax)
FAR_DATA_ENTRY(lib_tree, far_lib_tree)
FAR_DATA_ENTRY(lib_arities, far_lib_arities)
FAR_DATA_ENTRY(lib_shapes, far_lib_shapes)
FAR_DATA_ENTRY(lib_shape, far_lib_shape)
FAR_DATA_ENTRY(lib_kind_next, far_lib_kind_next)
FAR_DATA_ENTRY(lib_kind_prev, far_lib_kind_prev)
#endif
#ifdef FAR_ALIASES
#define lib_read far_lib_read
#define lib_open far_lib_open
#define lib_assertion_count far_lib_assertion_count
#define lib_symbol_counts far_lib_symbol_counts
#define lib_symbol_name far_lib_symbol_name
#define lib_label far_lib_label
#define lib_record far_lib_record
#define lib_conclusion far_lib_conclusion
#define lib_hypothesis far_lib_hypothesis
#define lib_sections far_lib_sections
#define lib_title_count far_lib_title_count
#define lib_title far_lib_title
#define lib_title_for far_lib_title_for
#define lib_glyph_map far_lib_glyph_map
#define lib_glyphs far_lib_glyphs
#define lib_glyph far_lib_glyph
#define lib_syntax_count far_lib_syntax_count
#define lib_syntax far_lib_syntax
#define lib_tree far_lib_tree
#define lib_arities far_lib_arities
#define lib_shapes far_lib_shapes
#define lib_shape far_lib_shape
#define lib_kind_next far_lib_kind_next
#define lib_kind_prev far_lib_kind_prev
#endif
#ifdef FAR_UNALIASES
#undef lib_read
#undef lib_open
#undef lib_assertion_count
#undef lib_symbol_counts
#undef lib_symbol_name
#undef lib_label
#undef lib_record
#undef lib_conclusion
#undef lib_hypothesis
#undef lib_sections
#undef lib_title_count
#undef lib_title
#undef lib_title_for
#undef lib_glyph_map
#undef lib_glyphs
#undef lib_glyph
#undef lib_syntax_count
#undef lib_syntax
#undef lib_tree
#undef lib_arities
#undef lib_shapes
#undef lib_shape
#undef lib_kind_next
#undef lib_kind_prev
#endif
