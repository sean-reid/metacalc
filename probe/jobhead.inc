; The first half of the image of an app that runs the check job: the header, the entry jump, the
; cross-page call and the far entries of the reader and the kernel, laid out as app/src/pages.inc
; lays out the app's three code pages, with the name jobchk, which header.inc's eight letters cannot hold.
; The including file defines VERSION and job_start, writes
; the entry page's code, then includes jobtail.inc, which fills the other two pages. The entry
; page makes every OS call. The kernel and the reader are on page 2, where the kernel's regions and
; the reader's state are those of ram_map.inc. An including file that defines JOB_OWN_KERNEL_ENTRIES
; writes the kernel's far entries itself.
#include "pages.inc"
.org 4000h
	.db 80h, 0Fh, 0, 0, 0, 0
	.db 80h, 12h, 01h, 04h
	.db 80h, 21h, VERSION
	.db 80h, 31h, 01h
	.db 80h, 46h, "jobchk"
	.db 80h, 81h, 01h
	.db 80h, 90h
	.db 03h, 26h, 09h, 04h, 04h, 6Fh, 1Bh, 80h
	.db 02h, 0Dh, 40h
	.db 0A1h, 6Bh, 99h, 0F6h, 59h, 0BCh, 67h, 0F5h, 85h, 9Ch, 09h, 6Ch, 0Fh, 0B4h, 03h, 9Bh
	.db 0C9h, 03h, 32h, 2Ch, 0E0h, 03h, 20h, 0E3h, 2Ch, 0F4h, 2Dh, 73h, 0B4h, 27h, 0C4h, 0A0h
	.db 72h, 54h, 0B9h, 0EAh, 7Ch, 3Bh, 0AAh, 16h, 0F6h, 77h, 83h, 7Ah, 0EEh, 1Ah, 0D4h, 42h
	.db 4Ch, 6Bh, 8Bh, 13h, 1Fh, 0BBh, 93h, 8Bh, 0FCh, 19h, 1Ch, 3Ch, 0ECh, 4Dh, 0E5h, 75h
	.db 80h, 7Fh, 0, 0, 0, 0
	.fill 4080h - $, 0
	jp job_start
#include "farcall.inc"
#define FAR_STUBS
#include "far_reader.inc"
#undef FAR_STUBS
#ifndef JOB_OWN_KERNEL_ENTRIES
#include "far_kernel.inc"
#endif
#include "libpage.inc"
