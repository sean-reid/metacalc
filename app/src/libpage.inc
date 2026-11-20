; Out: A = the flash page of library chunk 0, which sits on the page below the app's last code
; page, and the same value in libPage. APP_CODE_PAGES, default 1, is the app's number of code
; pages; the library follows them. Run on the entry page, whose page port 6 reads, before
; anything changes port 6.
#ifndef APP_CODE_PAGES
#define APP_CODE_PAGES 1
#endif

app_find_library:
	in a, (6)
	call app_lib_page
	ld (libPage), a
	ret

; In: A = the page the app was entered on.
app_lib_page:
	sub APP_CODE_PAGES
	ret
