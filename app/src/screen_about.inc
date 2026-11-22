; Kind 3, the program's About screen as About::program in crates/mcui/src/screens/about.rs draws
; it: the title bar, "metacalc" and the version on row 2, two fixed lines on rows 3 and 4, and the
; count of libraries on row 6, 1 when built with LIBRARY and 0 without. The version is
; VERSION_MAJOR.VERSION_MINOR.VERSION_PATCH of strings.inc. CLEAR and ENTER pop; every other key
; stays.

ABOUT_VERSION_ROW	.equ 2
ABOUT_LINE_ROW		.equ 3
ABOUT_COUNT_ROW		.equ 6

about_enter:
	xor a
	ret

about_draw:
	call far_ui_clear
	ld hl, str_about
	ld de, str_none
	call far_ui_title
	ld hl, str_about_version
	ld b, ABOUT_VERSION_ROW
	call far_ui_text_centered
	ld hl, str_about_a
	ld b, ABOUT_LINE_ROW
	call far_ui_text_centered
	ld hl, str_about_b
	ld b, ABOUT_LINE_ROW + 1
	call far_ui_text_centered
	ld hl, str_about_libs
	ld b, ABOUT_COUNT_ROW
	jp far_ui_text_centered

about_key:
	cp KEY_CLEAR
	jr z, about_pop
	cp KEY_ENTER
	jr z, about_pop
	ld a, SCR_STAY
	ret
about_pop:
	ld a, SCR_POP
	ret
