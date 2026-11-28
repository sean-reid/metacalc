; Code page 2, the data page: the kernel and the reader. Neither calls anything off the page. Its
; library constant UI_FONT_FINGERPRINT comes from the font on the draw page, which is assembled
; first. Included at the app's page 2 by metacalc.z80; see pages.inc.
	APP_PAGE(PAGE_DATA)
#include "farcall.inc"
#define KERNEL_EMBEDDED
#include "kernel.z80"
#define LIB_EMBEDDED
#include "lib/lib.z80"
#include "pw/pw.z80"
#ifdef LIBRARY
	APP_PAGE_END(PAGE_DATA)
#else
	APP_PAGE_LAST(PAGE_DATA)
#endif
