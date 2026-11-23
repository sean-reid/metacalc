; Code page 1, the draw page: the canvas and fm. fm calls the reader on the
; data page through the far entries of far_reader.inc, which the aliases name for the length of
; the include. Included at the app's page 1 by metacalc.z80; see pages.inc.
	APP_PAGE(PAGE_DRAW)
#include "farcall.inc"
#if $ != far_table
 .error "the far entries are not at the same address on every page"
#endif
#undef FAR_DATA_ENTRY
#define FAR_DATA_ENTRY(real, stub) FAR_JUMP_DATA(real)
#define FAR_STUBS
#include "far_reader.inc"
#undef FAR_STUBS
#if $ != far_table_end
 .error "the far entries are not the same size on every page"
#endif
#define UI_EMBEDDED
#include "ui.z80"
#if (UI_FONT_CH_39 - UI_FONT_CH_30) != 9
 .error "the digits 0 to 9 are not consecutive glyph codes"
#endif
#define FAR_ALIASES
#include "far_reader.inc"
#undef FAR_ALIASES
#define FM_EMBEDDED
#include "fm/fm.z80"
#define FAR_UNALIASES
#include "far_reader.inc"
#undef FAR_UNALIASES
	APP_PAGE_FILL(PAGE_DRAW)
