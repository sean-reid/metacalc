; Code page 1, the draw page: the canvas, fm and the shell's strings. fm calls the reader on the
; data page through the far entries of far_reader.inc, which the aliases name for the length of
; the include. Included at the app's page 1 by metacalc.z80; see pages.inc.
	APP_PAGE(PAGE_DRAW)
#define FAR_PAD
#include "farcall.inc"
#if $ != far_table
 .error "the far entries are not at the same address on every page"
#endif
#undef FAR_DATA_ENTRY
#define FAR_DATA_ENTRY(real, stub) FAR_CODE_TO_DATA(real)
#define FAR_STUBS
#include "far_reader.inc"
#undef FAR_STUBS
#if $ != far_table_end
 .error "the far entries are not the same size on every page"
#endif
#define UI_EMBEDDED
#include "ui.z80"
#define FAR_ALIASES
#include "far_reader.inc"
#undef FAR_ALIASES
#define FM_EMBEDDED
#include "fm/fm.z80"
#define FAR_UNALIASES
#include "far_reader.inc"
#undef FAR_UNALIASES
#include "strings.inc"
	APP_PAGE_FILL(PAGE_DRAW)
