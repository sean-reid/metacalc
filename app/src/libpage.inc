; Out: A = the flash page of library chunk 0, which sits on the page below the one the app
; was entered on, and the same value in libPage. Run before anything changes port 6.
app_find_library:
	in a, (6)
	call app_lib_page
	ld (libPage), a
	ret

; In: A = the page the app was entered on.
app_lib_page:
	dec a
	ret
