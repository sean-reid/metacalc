; The screen kinds the shell gives the screen stack (scr/scr.z80), the kind table and the routines
; that build a frame to push. The kind table has a row of five addresses, enter, draw, key, land and
; idle, for each kind. Kind 1 is in screen_libraries.inc, kind 2 in screen_notice.inc, kind 3 in
; screen_about.inc, kind 4 in screen_stage.inc, kind 5, checking, in screen_checking.inc, kind 6, the
; theorem list, in screen_list.inc, kind 7, the theorem, in screen_theorem.inc, kind 8, the sections of a
; stage, in screen_sections.inc, and kind 9, the theorem's distinct-variable conditions, in screen_dist.inc. Only
; kind 6 has a land, list_land in screen_list_land.inc, and every other row's land is screen_ret. Kinds 5 and 6
; have an idle routine, checking_idle and list_idle; the idle of the others is screen_idle.
;
; Frames, 16 bytes: byte 0 the kind, the rest 0 but what the kind keeps there.
;   KIND_LIBRARIES  byte 1 the cursor, byte 2 the first row shown
;   KIND_NOTICE     byte 1 the notice's number, byte 2 its one parameter byte
;   KIND_ABOUT      none
;   KIND_STAGE      byte 1 the stage
;   KIND_CHECKING   byte 1 the stage, byte 2 1 once its check has finished
;   KIND_LIST       byte 1 the stage, bytes 2 and 3 the cursor's assertion, 4 and 5 the first shown,
;                   byte 6 the kind of assertion shown,
;                   byte 7 the find's length and miss bit, bytes 8 to 15 its characters
;   KIND_THEOREM    byte 1 the stage, bytes 2 and 3 the assertion, 4 the cursor's statement, 5 its node,
;                   6 and 7 the first row shown, 8 following, 9 1 once entered
;   KIND_SECTIONS   byte 1 the stage, bytes 2 and 3 the cursor's title, 4 and 5 the first title shown
;   KIND_DIST       byte 1 the stage, bytes 2 and 3 the assertion, 4 and 5 the first row shown
;
; The routines run on this page, and so do their constant strings, which the canvas cannot read: a
; draw copies its screen's block (strings.inc) to APP_STAGE with app_stage_block and hands the canvas
; the copy's addresses, or the address of a string it built in APP_TEXT.

KIND_LIBRARIES	.equ 1
KIND_NOTICE	.equ 2
KIND_ABOUT	.equ 3
KIND_STAGE	.equ 4
KIND_CHECKING	.equ 5
KIND_LIST	.equ 6
KIND_THEOREM	.equ 7
KIND_SECTIONS	.equ 8
KIND_DIST	.equ 9
FRAME_CURSOR	.equ 1
FRAME_TOP	.equ 2

screen_kinds:
	.dw libs_enter, libs_draw, libs_key, screen_ret, screen_idle
	.dw notice_enter, notice_draw, notice_key, screen_ret, screen_idle
	.dw about_enter, about_draw, about_key, screen_ret, screen_idle
	.dw stage_enter, stage_draw, about_key, screen_ret, screen_idle
	.dw checking_enter, checking_draw, checking_key, screen_ret, checking_idle
	.dw list_enter, list_draw, list_key, list_land, list_idle
	.dw theorem_enter, theorem_draw, theorem_key, screen_ret, screen_idle
	.dw sections_enter, sections_draw, sections_key, screen_ret, screen_idle
	.dw dist_enter, dist_draw, dist_key, screen_ret, screen_idle

screen_first:
	.db KIND_LIBRARIES
	.fill 15, 0

screen_ret:
	ret

; Out: A = SCR_STAY, carry clear: nothing pending.
screen_idle:
	xor a
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
