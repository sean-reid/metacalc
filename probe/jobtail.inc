; The second half of the image: the entry page is filled, page 1 holds only the cross-page call, and
; page 2 holds the kernel and the reader. A library, if the including file defines LIBRARY and
; appends one, starts at the page after; without one, the signature follows the last page.
	APP_PAGE_FILL(PAGE_ENTRY)
	APP_PAGE(PAGE_DRAW)
#include "farcall.inc"
	APP_PAGE_FILL(PAGE_DRAW)
	APP_PAGE(PAGE_DATA)
#include "farcall.inc"
#ifndef LIB_FONT_FINGERPRINT
#define LIB_FONT_FINGERPRINT 0
#endif
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
