; The theorem screen's draw (screen_theorem.inc has the kind's header): the title with the mark and
; "thm" or "ax", each statement's rows inside the window by fm_draw with no fit, the numbers of the
; hypotheses in the gutter, the marker or the cursor, and the keys. Strings built here are in FM_SCRATCH
; and the constants are staged from theorem_block. Every routine here may destroy AF BC DE HL and IX.

#ifdef LIBRARY
; In: HL = the frame.
theorem_draw:
	ld de, 4
	add hl, de
	ld de, TV_S
	ld bc, 5
	ldir
	call far_ui_clear
	ld hl, theorem_block
	ld bc, THEOREM_BLOCK_SIZE
	call app_stage_block
	ld hl, TV_CUT
	ld de, TV_RIGHT
	call far_ui_title
	ld a, (TV_LARGE)
	or a
	jr nz, td_large
	xor a
	ld (TV_I), a
td_next:
	call theorem_statement
	ld hl, TV_I
	inc (hl)
	ld a, (TV_NHYPS)
	cp (hl)
	jr nc, td_next
	ld hl, STG_TH_KEYS
	jp far_ui_soft_keys
td_large:
	ld hl, STG_TH_LINE_1
	ld b, THM_LARGE_ROW
	call far_ui_text_centered
	ld hl, STG_TH_LINE_2
	ld b, THM_LARGE_ROW + 1
	call far_ui_text_centered
	ld hl, STG_TH_LINE_3
	ld b, THM_LARGE_ROW + 2
	call far_ui_text_centered
	ld hl, STG_TH_KEYS_LARGE
	jp far_ui_soft_keys

; Builds the title: TV_RIGHT holds the mark and "thm" or "ax", TV_CUT the label cut to the room they leave.
theorem_title:
	ld hl, (TV_ASSERT)
	ld de, TV_LABEL
	call list_label_text
	or a
	jr z, tt_label
	xor a
	ld (TV_LABEL), a
tt_label:
	ld b, 0
	ld a, (TV_TABLE)
	or a
	jr z, tt_mark
	ld hl, (TV_ASSERT)
	call store_get
	or a
	jr z, tt_mark
	ld b, 0
tt_mark:
	ld c, b
	ld b, 0
	ld hl, list_marks
	add hl, bc
	ld a, (hl)
	ld de, TV_RIGHT
	ld (de), a
	inc de
	ld a, UI_FONT_CH_20
	ld (de), a
	inc de
	push de
	ld hl, (TV_ASSERT)
	call far_lib_record
	ld hl, theorem_ax
	or a
	jr nz, tt_kind
	ld a, (lr_dkind)
	or a
	jr z, tt_kind
	ld hl, theorem_thm
tt_kind:
	pop de
	call app_strcpy
	ld hl, TV_RIGHT
	call list_room
	ld hl, TV_LABEL
	ld de, TV_CUT
	ld b, LIST_LABEL_MAX + 2
	jp far_ui_truncate

; Draws statement TV_I: its number in the gutter for a hypothesis, its rows inside the window with the
; cursor's pieces, and the marker when the cursor is on its node 0.
theorem_statement:
	ld a, (TV_I)
	add a, a
	ld e, a
	ld d, 0
	ld hl, TV_STARTS
	add hl, de
	ld e, (hl)
	inc hl
	ld d, (hl)
	inc hl
	ld c, (hl)
	inc hl
	ld b, (hl)
	ld hl, (TV_SCROLL)
	or a
	sbc hl, bc
	ret nc
	ld hl, (TV_SCROLL)
	ld bc, THM_ROWS
	add hl, bc
	or a
	sbc hl, de
	ret z
	ret c
	ld hl, (TV_SCROLL)
	ex de, hl
	or a
	sbc hl, de
	inc hl
	ld (TV_ROW), hl
	ld a, (TV_I)
	call theorem_select
	ret nz
	ld a, (TV_I)
	ld hl, TV_NHYPS
	cp (hl)
	ld a, THM_GUTTER
	jr z, ts_x
	ld hl, TV_INDENT
	add a, (hl)
ts_x:
	ld (TV_X), a
	ld a, (TV_I)
	ld hl, TV_NHYPS
	cp (hl)
	jr z, ts_cursor
	call theorem_in_window
	jr nz, ts_cursor
	ld a, (TV_I)
	inc a
	ld l, a
	ld h, 0
	ld de, TV_NUMBER
	call app_dec16
	ld a, UI_FONT_CH_2E
	ld (de), a
	inc de
	xor a
	ld (de), a
	ld hl, TV_NUMBER
	ld de, THM_GUTTER
	ld a, (TV_ROW)
	ld b, a
	call far_ui_text
ts_cursor:
	ld a, (TV_I)
	ld hl, TV_S
	cp (hl)
	jr nz, ts_draw
	ld a, (TV_N)
	or a
	jr z, ts_draw
	call far_fm_cursor
ts_draw:
	ld a, (TV_I)
	call theorem_width
	ld a, (TV_X)
	ld e, a
	ld hl, (TV_ROW)
	ld bc, (1 << 8) | (THM_ROWS + 1)
	call far_fm_draw
	ld a, (TV_I)
	ld hl, TV_S
	cp (hl)
	ret nz
	ld a, (TV_N)
	or a
	ret nz
	call theorem_in_window
	ret nz
	ld a, (TV_ROW)
	jp far_ui_marker

; Out: Z when TV_ROW is a content row, 1 to 6. Destroys AF.
theorem_in_window:
	ld a, (TV_ROW + 1)
	or a
	ret nz
	ld a, (TV_ROW)
	dec a
	cp THM_ROWS
	jr nc, tw_out
	xor a
	ret
tw_out:
	or 1
	ret

theorem_thm:
	.db UI_FONT_CH_74, UI_FONT_CH_68, UI_FONT_CH_6D, 0
theorem_ax:
	.db UI_FONT_CH_61, UI_FONT_CH_78, 0
#else
theorem_draw:
	xor a
	ret
#endif
