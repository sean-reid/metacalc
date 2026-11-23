; The screen kinds the shell gives the screen stack (scr/scr.z80), the kind table and the routines
; that build a frame to push. The kind table has a row of four addresses, enter, draw, key and land,
; for each kind. Kind 1 is in screen_libraries.inc, kind 2 in screen_notice.inc, kind 3 in
; screen_about.inc and kind 4 in screen_stage.inc. No kind has a land: a row's land is a ret.
;
; Frames, 16 bytes: byte 0 the kind, the rest 0 but what the kind keeps there.
;   KIND_LIBRARIES  byte 1 the cursor, byte 2 the first row shown
;   KIND_NOTICE     byte 1 the notice's number, byte 2 its one parameter byte
;   KIND_ABOUT      none
;   KIND_STAGE      byte 1 the stage
;
; The routines run on this page, and so do their constant strings, which the canvas cannot read: a
; draw copies its screen's block (strings.inc) to APP_STAGE with app_stage_block and hands the canvas
; the copy's addresses, or the address of a string it built in APP_TEXT.

KIND_LIBRARIES	.equ 1
KIND_NOTICE	.equ 2
KIND_ABOUT	.equ 3
KIND_STAGE	.equ 4
FRAME_CURSOR	.equ 1
FRAME_TOP	.equ 2

screen_kinds:
	.dw libs_enter, libs_draw, libs_key, screen_ret
	.dw notice_enter, notice_draw, notice_key, screen_ret
	.dw about_enter, about_draw, about_key, screen_ret
	.dw stage_enter, stage_draw, about_key, screen_ret

screen_first:
	.db KIND_LIBRARIES
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
	ld c, 0

; In: A = the kind, B = the number, C = the parameter. As screen_push.
screen_push_param:
	ld hl, scr_new
	call screen_make
	ld a, SCR_PUSH
	ret
