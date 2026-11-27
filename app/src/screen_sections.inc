; Kind 8, the sections screen as Sections in crates/mcui/src/screens/sections.rs draws it: the stage's name
; and "cursor/count" in the title bar, six rows of the stage's section titles from the first one shown, each
; indented by its depth less one (at most three spaces) and cut to the row, the cursor's row inverted, and
; the key Go. The titles are the library's first N, those whose first assertion is below the stage's end, so
; a title number here is the library's.
;
; Frame: byte 1 the stage, bytes 2 and 3 the cursor's title, bytes 4 and 5 the first title shown, little
; endian, both below the count. A library's title count is a u16, so a stage can have more than 255 titles
; and each number is two bytes. Position only: sections_push writes all of it from the list's frame. The
; enter that runs after an auto power down or after the error notice pops derives the stage table and the
; count and writes no byte of the frame, so the cursor and the first title stay where they were.
;
; Derived state, in APP_TEXT:
;   SX_N        the stage's title count, from lib_title_for at the stage's last assertion, plus one
;   SX_STAGE, SX_CUR, SX_TOP, SX_I, SX_ROW   the draw's copies of the frame's numbers, the title being
;               drawn and its row, written by sections_draw before it reads them
;   SX_LEN, SX_USED, SX_PAD   the row's text length (at most 48), how many of its characters the first
;               pass converted (at most 24: a row that holds more is converted again whole when they fit
;               the row) and its indent
;   SX_RIGHT    "cursor/count" as glyph codes, SX_NAME the stage's name cut to its room, SX_TEXT a row
; The count is the only state a key reads that the frame lacks. The enter builds it again, with the stage
; table (APP_LIST) it reads the stage's end from, through libs_list: nothing of APP_COUNTS, APP_CRC or
; OP1 to OP5 is read or written by this screen's enter, key, draw or idle, none of which asks the store.
; The push asks it under Unchecked with a table, as the list's own keys do, since lf_next reads the
; status table. APP_TEXT is written while the screen is on top by the draw, by the error notice that a
; refused title read pushes over it, whose pop runs this screen's enter again, and by the OS's overwrite
; of saveSScreen at an auto power down, which the enter repairs. The screen calls no fm routine. The list
; underneath keeps its preview row in fm's memory, which only a kernel entry overwrites, and its APP_TEXT
; state is derived again by list_enter, which makes the preview pending again, when this screen goes. Titles
; are read through FM_SCRATCH, which holds nothing between calls.
;
; Registers: each routine below may destroy AF BC DE HL and IX.
; sections_push  in: HL = a list's frame, from list_key's F5 with LV_SECT 1. Fills scr_new with the frame
;                for the list's stage and the cursor's section: the title lib_title_for gives for the
;                cursor's assertion, title 0 when none starts at or before it or the reader refuses, and
;                title 0 for a list that shows nothing (under a kind, when lf_next does not answer the
;                cursor); the first title shown is the model's reveal(0, cursor, 6), the cursor less 5 or
;                0. Out: A = SCR_PUSH. Writes scr_new, LK_FR, LK_X and LK_T, and under a kind LV_KIND,
;                LV_END and, for Unchecked with a table, LK_AGAIN. What it asks for writes more: the
;                reader's LIB_STATE and FAR_VARS, and OP1 to OP5 under Unchecked with a table (the store).
; sections_enter in: HL = the frame. Derives APP_LIST again with libs_list, since the stage's end is read
;                from it and an auto power down overwrites it. Out: A = 0 after writing SX_N, else the
;                status of libs_list or lib_title_for, 9 for a stage with no title. Writes APP_LIST, SX_N
;                and the reader's LIB_STATE and FAR_VARS, and no other byte of the frame or the store's.
; sections_key   in: A = the key, HL = the frame. UP, DOWN, LEFT and RIGHT move the cursor by one or six
;                titles, never past either end, and move the first title as little as the model's reveal
;                does, answering SCR_REDRAW and writing bytes 2 and 3 of the frame, and 4 and 5 when the
;                first title moves; one that moves nothing answers SCR_STAY with the frame and the
;                variables untouched. ENTER and F1 answer SCR_JUMP with DE = the cursor's title's first
;                assertion, writing FM_SCRATCH, LIB_STATE and FAR_VARS, or push the error notice with the
;                reader's status when the read is refused, writing scr_new too. CLEAR pops. Every other
;                key stays and writes nothing.
; sections_draw  in: HL = the frame. Clears the canvas, stages sections_block, and draws the title, the
;                rows and the key. A title the reader refuses is drawn as an empty row. Writes SX_STAGE,
;                SX_CUR, SX_TOP, SX_I, SX_ROW, SX_LEN, SX_USED, SX_PAD, SX_RIGHT, SX_NAME, SX_TEXT,
;                FM_SCRATCH, the canvas (UI_FRAME and UI_VARS), the reader's LIB_STATE and FAR_VARS, and
;                APP_STAGE when it does not hold this screen's block already.
; Idle is screen_idle and land is screen_ret.
; Interrupts and speed: every routine returns with both as at the call, at every exit, and the screen
; writes no port 20h and executes no DI; the reader's reads turn interrupts off for their own length and
; give the caller's state back.
; Stack, in bytes written below the stack pointer at the entry, the caller's return address counted, the
; greatest over a call with interrupts masked, at both speeds (sections_depth.rs). Stub OS: sections_push
; 18 under All, Theorems and Axioms, 22 under Unchecked with a table or when it shows nothing, 8 when the
; reader refuses (the same as list_key's F5, which jumps to it); sections_enter 18, 8 when a read is
; refused; sections_key 4 for a move, a stay and CLEAR, 18 for ENTER and F1, 8 when the read is refused;
; sections_draw 26. Reference OS: sections_push 18 and 36, 8 refused; sections_enter 18 and 8;
; sections_key 4, 18 and 8; sections_draw 26.

SX_ROWS		.equ 6
SX_INDENT_MAX	.equ 3
SX_TEXT_MAX	.equ 48
SX_WINDOW	.equ 24
SX_TEXT_SIZE	.equ SX_INDENT_MAX + SX_TEXT_MAX + 1
SX_ROW_WIDTH	.equ 94

SX_N		.equ APP_TEXT
SX_CUR		.equ APP_TEXT + 2
SX_TOP		.equ APP_TEXT + 4
SX_I		.equ APP_TEXT + 6
SX_STAGE	.equ APP_TEXT + 8
SX_ROW		.equ APP_TEXT + 9
SX_LEN		.equ APP_TEXT + 10
SX_USED		.equ APP_TEXT + 11
SX_PAD		.equ APP_TEXT + 12
SX_RIGHT	.equ APP_TEXT + 13
SX_NAME		.equ SX_RIGHT + 12
SX_TEXT		.equ SX_NAME + LIST_NAME_SIZE
SX_END		.equ SX_TEXT + SX_TEXT_SIZE
#if SX_END > (APP_TEXT + APP_TEXT_SIZE)
 .error "the sections screen's variables are longer than APP_TEXT"
#endif

#ifdef LIBRARY
sections_push:
	ld (LK_FR), hl
	push hl
	pop ix
	ld a, (ix + 6)
	or a
	jr z, sp_cursor
	ld (LV_KIND), a
	ld a, (ix + 1)
	call list_stage_end
	ld (LV_END), de
	ld l, (ix + 2)
	ld h, (ix + 3)
	call lf_next
	ld ix, (LK_FR)
	ld e, (ix + 2)
	ld d, (ix + 3)
	or a
	sbc hl, de
	jr nz, sp_nothing
sp_cursor:
	ld l, (ix + 2)
	ld h, (ix + 3)
	call far_lib_title_for
	or a
	jr z, sp_have
sp_nothing:
	ld hl, 0
sp_have:
	ld (LK_X), hl
	ld de, 1 - SX_ROWS
	add hl, de
	jr c, sp_top
	ld hl, 0
sp_top:
	ld (LK_T), hl
	ld ix, (LK_FR)
	ld b, (ix + 1)
	ld a, KIND_SECTIONS
	call screen_push
	ld hl, (LK_X)
	ld (scr_new + 2), hl
	ld hl, (LK_T)
	ld (scr_new + 4), hl
	ret

; In: HL = the frame.
sections_enter:
	push hl
	call libs_list
	pop hl
	or a
	ret nz
	inc hl
	ld a, (hl)
	call list_stage_end
	dec de
	ex de, hl
	call far_lib_title_for
	or a
	ret nz
	inc hl
	ld (SX_N), hl
	xor a
	ret

; In: A = the key, HL = the frame.
sections_key:
	push hl
	pop ix
	cp KEY_UP
	jr z, sxk_up
	cp KEY_DOWN
	jr z, sxk_down
	cp KEY_LEFT
	jr z, sxk_left
	cp KEY_RIGHT
	jr z, sxk_right
	cp KEY_ENTER
	jr z, sxk_go
	cp KEY_F1
	jr z, sxk_go
	cp KEY_CLEAR
	ld a, SCR_POP
	ret z
sxk_stay:
	ld a, SCR_STAY
	ret
sxk_up:
	ld bc, 1
	jr sxk_back
sxk_left:
	ld bc, SX_ROWS
sxk_back:
	ld l, (ix + 2)
	ld h, (ix + 3)
	or a
	sbc hl, bc
	jr nc, sxk_moved
	ld hl, 0
	jr sxk_moved
sxk_down:
	ld bc, 1
	jr sxk_fwd
sxk_right:
	ld bc, SX_ROWS
sxk_fwd:
	ld hl, (SX_N)
	dec hl
	ex de, hl
	ld l, (ix + 2)
	ld h, (ix + 3)
	add hl, bc
	jr c, sxk_last
	or a
	sbc hl, de
	jr c, sxk_inside
sxk_last:
	ex de, hl
	jr sxk_moved
sxk_inside:
	add hl, de
sxk_moved:
	ld a, (ix + 2)
	cp l
	jr nz, sxk_set
	ld a, (ix + 3)
	cp h
	jr z, sxk_stay
sxk_set:
	ld (ix + 2), l
	ld (ix + 3), h
	ld e, (ix + 4)
	ld d, (ix + 5)
	or a
	sbc hl, de
	jr c, sxk_above
	ld a, h
	or a
	jr nz, sxk_below
	ld a, l
	cp SX_ROWS
	jr c, sxk_redraw
sxk_below:
	add hl, de
	ld de, 1 - SX_ROWS
	add hl, de
	jr sxk_top
sxk_above:
	add hl, de
sxk_top:
	ld (ix + 4), l
	ld (ix + 5), h
sxk_redraw:
	ld a, SCR_REDRAW
	ret
sxk_go:
	ld l, (ix + 2)
	ld h, (ix + 3)
	ld de, FM_SCRATCH
	call far_lib_title
	or a
	jr nz, sxk_fail
	ex de, hl
	ld a, SCR_JUMP
	ret
sxk_fail:
	ld c, a
	ld b, NOTICE_ERROR
	ld a, KIND_NOTICE
	jp screen_push_param

; In: HL = the frame.
sections_draw:
	inc hl
	ld a, (hl)
	ld (SX_STAGE), a
	inc hl
	ld e, (hl)
	inc hl
	ld d, (hl)
	ld (SX_CUR), de
	inc hl
	ld e, (hl)
	inc hl
	ld d, (hl)
	ld (SX_TOP), de
	ld (SX_I), de
	call far_ui_clear
	ld hl, sections_block
	ld bc, SECTIONS_BLOCK_SIZE
	call app_stage_block
	call sections_title
	xor a
	ld (SX_ROW), a
sd_row:
	ld hl, (SX_I)
	ld de, (SX_N)
	or a
	sbc hl, de
	jr nc, sd_keys
	call sections_row
	ld hl, (SX_I)
	inc hl
	ld (SX_I), hl
	ld hl, SX_ROW
	inc (hl)
	ld a, (hl)
	cp SX_ROWS
	jr c, sd_row
sd_keys:
	ld hl, STG_SX_KEYS
	jp far_ui_soft_keys

; Draws the title bar: the stage's name cut to the room "cursor/count" leaves.
sections_title:
	ld hl, (SX_CUR)
	inc hl
	ld de, SX_RIGHT
	call app_dec16
	ld a, UI_FONT_CH_2F
	ld (de), a
	inc de
	ld hl, (SX_N)
	call app_dec16
	xor a
	ld (de), a
	ld a, (SX_STAGE)
	call stage_strings
	ld de, SX_NAME
	call app_strcpy
	ld hl, SX_RIGHT
	call list_room
	ld hl, SX_NAME
	ld d, h
	ld e, l
	ld b, LIST_NAME_SIZE
	call far_ui_truncate
	ld hl, SX_NAME
	ld de, SX_RIGHT
	jp far_ui_title

; Draws the row SX_ROW + 1 for the title SX_I: its depth less one in spaces, at most SX_INDENT_MAX, then
; its first SX_TEXT_MAX characters, cut to the row; empty when the reader refuses the title.
sections_row:
	ld hl, (SX_I)
	ld de, FM_SCRATCH
	call far_lib_title
	ld de, SX_TEXT
	or a
	jr nz, sr_blank
	ld a, c
	sub 1
	jr nc, sr_depth
	xor a
sr_depth:
	cp SX_INDENT_MAX + 1
	jr c, sr_indent
	ld a, SX_INDENT_MAX
sr_indent:
	ld c, a
	ld a, b
	cp SX_TEXT_MAX + 1
	jr c, sr_pad
	ld a, SX_TEXT_MAX
sr_pad:
	ld (SX_LEN), a
	ld a, c
	ld (SX_PAD), a
	ld a, (SX_LEN)
	cp SX_WINDOW + 1
	jr c, sr_build
	ld a, SX_WINDOW
sr_build:
	ld b, a
	ld (SX_USED), a
	ld de, SX_TEXT
	ld a, (SX_PAD)
	ld c, a
	or a
	jr z, sr_text
sr_space:
	ld a, UI_FONT_CH_20
	ld (de), a
	inc de
	dec c
	jr nz, sr_space
sr_text:
	ld hl, FM_SCRATCH
	call ll_glyph
	call sr_cut
	ld a, (SX_USED)
	ld b, a
	ld a, (SX_LEN)
	cp b
	jr z, sr_row
	ld hl, SX_TEXT
sr_end:
	ld a, (hl)
	inc hl
	or a
	jr nz, sr_end
	dec hl
	dec hl
	ld a, (hl)
	cp UI_FONT_CH_2026
	jr z, sr_row
	ld a, (SX_LEN)
	jr sr_build
sr_blank:
	xor a
	ld (de), a
	call sr_cut
sr_row:
	ld hl, (SX_I)
	ld de, (SX_CUR)
	or a
	sbc hl, de
	ld c, 0
	jr nz, sr_draw
	inc c
sr_draw:
	ld a, (SX_ROW)
	inc a
	ld b, a
	ld hl, SX_TEXT
	ld de, STG_SX_NONE
	jp far_ui_list_row

sr_cut:
	ld hl, SX_TEXT
	ld d, h
	ld e, l
	ld b, SX_TEXT_SIZE
	ld c, SX_ROW_WIDTH
	jp far_ui_truncate
#else
sections_enter:
sections_draw:
	xor a
	ret
sections_key:
	xor a
	ret
#endif
