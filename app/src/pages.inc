; The app's three code pages and the far entries between them. The pages are assembled in this
; order, each at 4000h, and the library follows the last:
;   0  entry  header, shell, screens, app_key and every OS call; the far entries of the other two
;   1  draw   the canvas (ui/), fm (fm/) and the shell's constant strings; the far entries of the
;             reader
;   2  data   the kernel (kernel/) and the reader (lib/)
; A routine on one page calls a routine on another through its far entry, a stub on the caller's
; page that loads IX and jumps to farcall.inc's far_pageN for the callee's page N. FAR_LABEL_TO_DRAW and
; FAR_LABEL_TO_DATA are the only places a page number is written next to a routine, and APP_PAGE refuses
; a page that is not assembled in its turn. Kernel, reader and canvas call nothing off their page,
; and no routine off the entry page makes an OS call.
;
; A far call destroys IX, which the reader's own contract keeps: a caller across pages holds
; nothing in IX over the call.
;
; Pointers: a string or table handed to a routine on another page is in RAM or on that page, since
; the callee runs with its own page mapped. The shell's strings are on the draw page, and the
; kernel_init parameter block is in RAM.
;
; APP_PAGE_END(n) stops the assembly when page n runs past 4000h to 7FFFh, which spasm does not do
; by itself, and prints the bytes the page uses. APP_PAGE_FILL(n) does the same and then pads the
; page to its end.

#define APP_CODE_PAGES 3
PAGE_ENTRY      = 0
PAGE_DRAW       = 1
PAGE_DATA       = 2

#macro FAR_LABEL_TO_DRAW(real, stub)
stub .equ $
	ld ix, real
	jp far_page1
#endmacro

#macro FAR_LABEL_TO_DATA(real, stub)
stub .equ $
	ld ix, real
	jp far_page2
#endmacro

#macro FAR_CODE_TO_DATA(real)
	ld ix, real
	jp far_page2
#endmacro

; A list line is FAR_DRAW_ENTRY(routine, stub) or FAR_DATA_ENTRY(routine, stub). A stub is a label
; and its code the first time a list is included. A page that repeats a list defines
; FAR_DATA_ENTRY as FAR_CODE_TO_DATA, which writes the same code without a label, at the same
; address as the first page, so one label serves every page that has the stub.
#define FAR_DRAW_ENTRY(real, stub) FAR_LABEL_TO_DRAW(real, stub)
#define FAR_DATA_ENTRY(real, stub) FAR_LABEL_TO_DATA(real, stub)

#define PAGE_NEXT 1

#macro APP_PAGE(n)
#if n != PAGE_NEXT
 .error "the code pages are not assembled in order"
#endif
#define PAGE_NEXT eval(n + 1)
.org 4000h
#endmacro

#macro APP_PAGE_END(n)
#if $ > 8000h
 .error "a code page is over 16384 bytes"
#endif
.echo "code page ", n, ": ", $ - 4000h, " of 16384 bytes\n"
#endmacro

#macro APP_PAGE_FILL(n)
APP_PAGE_END(n)
.fill 8000h - $, 0
#endmacro
