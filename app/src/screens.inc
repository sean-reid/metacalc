; The screen kinds the shell gives the screen stack (scr/scr.z80) and the start screen, kind 1.
; The kind table has a row of four addresses, enter, draw, key and land, for each kind. Kinds 2 and
; 3 are in screen_notice.inc and screen_about.inc. No kind has a land: a row's land is a ret.
;
; Frames, 16 bytes: byte 0 the kind, byte 1 the notice's number, byte 2 its one parameter byte,
; the rest 0. KIND_START and KIND_ABOUT use no other byte.
;
; The routines run on this page, where the strings they draw are not: a string goes to the canvas's
; far entries by the address it has on the draw page (strings.inc), or by its address in APP_BUF.

KIND_START	.equ 1
KIND_NOTICE	.equ 2
KIND_ABOUT	.equ 3
FRAME_NUMBER	.equ 1
FRAME_PARAM	.equ 2

; The frame the error notice is pushed from, outside SCR_STACK.
APP_FRAME	.equ APP_BUF + $80

screen_kinds:
	.dw start_enter, start_draw, start_key, screen_ret
	.dw notice_enter, notice_draw, notice_key, screen_ret
	.dw about_enter, about_draw, about_key, screen_ret

screen_first:
	.db KIND_START
	.fill 15, 0

screen_ret:
	ret

; In: HL = a frame, A = kind, B = number, C = parameter. Writes them and zeros the rest.
; Out: HL = the frame. Destroys AF BC DE.
screen_make:
	push hl
	ld (hl), a
	inc hl
	ld (hl), b
	inc hl
	ld (hl), c
	inc hl
	ld d, h
	ld e, l
	inc de
	ld (hl), 0
	ld bc, SCR_FRAME - 4
	ldir
	pop hl
	ret

; In: A = the kind, B = the number. Fills scr_new with that frame for a key routine to push.
; Out: A = SCR_PUSH.
screen_push:
	ld hl, scr_new
	ld c, 0
	call screen_make
	ld a, SCR_PUSH
	ret

; In: A = the key. The keys of the start screen, the table crates/mcemu/tests/app_pages/screens.rs
; checks for every key the app can produce: F3 pushes About, F1 and ENTER the notice of the
; theorems, F2 the notice of the verifier, and CLEAR pops, which leaves the app from the last
; screen. UP, DOWN, LEFT, RIGHT, F4, F5, the letters and KEY_NONE stay.
start_key:
	cp KEY_F3
	jr z, start_about
	cp KEY_F1
	jr z, start_theorems
	cp KEY_ENTER
	jr z, start_theorems
	cp KEY_F2
	jr z, start_verify
	cp KEY_CLEAR
	ld a, SCR_STAY
	ret nz
	ld a, SCR_POP
	ret
start_about:
	ld a, KIND_ABOUT
	ld b, 0
	jr screen_push
start_theorems:
	ld b, NOTICE_THEOREMS
	jr start_notice
start_verify:
	ld b, NOTICE_VERIFY
start_notice:
	ld a, KIND_NOTICE
	jr screen_push

; Builds what the start screen draws from: with LIBRARY the first assertion in fm, a status that is
; not 0 kept in appStatus for start_draw to show. Out: A = 0, always.
start_enter:
#ifdef LIBRARY
	ld a, (appStatus)
	or a
	jr nz, start_built
	ld hl, 0
	ld de, APP_BUF
	call far_lib_label
	or a
	jr nz, start_kept
	ld hl, 0
	call far_fm_assertion
	or a
	jr z, start_built
start_kept:
	ld (appStatus), a
start_built:
#endif
	xor a
	ret

; Draws the title and, with LIBRARY, rows 1 to 7 of the library screen, without it the hint.
start_draw:
	call far_ui_clear
	ld hl, str_title
	ld de, str_none
	call far_ui_title
#ifdef LIBRARY
	ld a, (appStatus)
	or a
	jr nz, start_refused
	ld hl, 0
	ld de, APP_BUF
	call far_lib_label
	or a
	jr nz, start_failed
	ld hl, APP_BUF
	call app_codes
	ld hl, FORMULA_ROW
	ld d, FORMULA_WIDTH
	ld e, FORMULA_X
	ld b, FORMULA_ROW
	ld c, FORMULA_END
	call far_fm_draw
	or a
	jr nz, start_failed
	ld hl, APP_BUF
	ld de, HINT_COL
	ld b, LABEL_ROW
	call far_ui_text
	ld hl, str_hint
	ld de, HINT_COL
	ld b, LIBRARY_HINT_ROW
	jp far_ui_text
start_failed:
	ld (appStatus), a
; Shows appStatus as two hex digits on the hint row.
start_refused:
	ld a, (appStatus)
	ld hl, APP_BUF
	call app_hex_byte
	ld hl, APP_BUF
	ld de, HINT_COL
	ld b, LIBRARY_HINT_ROW
	jp far_ui_text
#else
	ld hl, str_hint
	ld de, HINT_COL
	ld b, HINT_ROW
	jp far_ui_text
#endif
