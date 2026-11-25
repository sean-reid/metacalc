; The list screen's draw (screen_list.inc has the kind's header): the title with the stage's name cut to
; the room checked/total leaves, the rows from the top, the preview and the keys. Strings built here are
; in APP_TEXT and the constants are staged from list_block.

#ifdef LIBRARY
; In: HL = the frame.
list_draw:
	inc hl
	ld a, (hl)
	ld (LV_STAGE), a
	inc hl
	ld e, (hl)
	inc hl
	ld d, (hl)
	ld (LV_CUR), de
	inc hl
	ld e, (hl)
	inc hl
	ld d, (hl)
	ld (LV_TOP), de
	call far_ui_clear
	ld hl, list_block
	ld bc, LIST_BLOCK_SIZE
	call app_stage_block
	call list_title
	ld a, (LV_STAGE)
	call list_stage_end
	ld (LV_END), de
	ld a, d
	or e
	jr nz, ld_rows
	ld hl, STG_LIST_NONE
	ld b, 3
	call far_ui_text_centered
	jr ld_preview
ld_rows:
	xor a
	ld (LV_ROW), a
	ld hl, (LV_TOP)
	ld (LV_AT), hl
ld_row:
	ld hl, (LV_AT)
	ld de, (LV_END)
	or a
	sbc hl, de
	jr nc, ld_preview
	call list_row
	ld hl, (LV_AT)
	inc hl
	ld (LV_AT), hl
	ld hl, LV_ROW
	inc (hl)
	ld a, (hl)
	cp LIST_ROWS
	jr c, ld_row
ld_preview:
	ld a, (LV_PREVIEW)
	cp LVP_ROW
	jr nz, ld_keys
	ld hl, LIST_PREVIEW_ROW
	ld d, LIST_WIDTH
	ld e, 1
	ld b, LIST_PREVIEW_ROW
	ld c, LIST_PREVIEW_ROW + 1
	call far_fm_draw
ld_keys:
	ld hl, STG_LIST_KEYS
	ld a, (LV_SECT)
	or a
	jr nz, ld_soft
	ld hl, STG_LIST_KEYS_PLAIN
ld_soft:
	jp far_ui_soft_keys

; In: HL = the right text of a row, glyph codes ended by 0, empty for none. Out: C = the pixels left
; of it for the row's left text: 94 for no text, else 91 less its width, 0 at the least. Destroys
; AF B HL.
list_room:
	ld a, (hl)
	push af
	call far_ui_text_width
	pop af
	ld b, 94
	or a
	jr z, lm_room
	ld b, 91
lm_room:
	ld a, b
	sub l
	jr nc, lm_fit
	xor a
lm_fit:
	ld c, a
	ret

; Draws the title bar: the stage's name, cut to the room the counts leave, and "checked/total" with the
; model's "flat " in front when LV_FLAT is 1.
list_title:
	ld a, (LV_STAGE)
	call libs_figures
	push de
	push hl
	ld de, LV_RIGHT
	ld a, (LV_FLAT)
	or a
	jr z, lt_counts
	ld hl, lq_flat
	call app_strcpy
	dec de
lt_counts:
	pop bc
	pop hl
	push bc
	call app_dec16
	ld a, UI_FONT_CH_2F
	ld (de), a
	inc de
	pop hl
	call app_dec16
	xor a
	ld (de), a
	ld a, (LV_STAGE)
	call stage_strings
	ld de, LV_NAME
	call app_strcpy
	ld hl, LV_RIGHT
	call list_room
	ld hl, LV_NAME
	ld de, LV_CUT
	ld b, LIST_NAME_SIZE
	call far_ui_truncate
	ld hl, LV_CUT
	ld de, LV_RIGHT
	jp far_ui_title

; Draws the row LV_ROW + 1 for the assertion LV_AT: its label cut to the room its mark leaves, the mark
; and the cursor's inversion.
list_row:
	ld hl, (LV_AT)
	ld de, LV_LABEL
	call app_label_text
	or a
	jr z, lr_label
	xor a
	ld (LV_LABEL), a
lr_label:
	ld b, 0
	ld a, (LV_TABLE)
	or a
	jr z, lr_mark
	ld hl, (LV_AT)
	call store_get
	or a
	jr z, lr_mark
	ld b, 0
lr_mark:
	ld c, b
	ld b, 0
	ld hl, list_marks
	add hl, bc
	ld a, (hl)
	ld (LV_MARK), a
	xor a
	ld (LV_MARK + 1), a
	ld hl, LV_MARK
	call list_room
	ld hl, LV_LABEL
	ld de, LV_TEXT
	ld b, APP_LABEL_MAX + 2
	call far_ui_truncate
	ld hl, (LV_AT)
	ld de, (LV_CUR)
	or a
	sbc hl, de
	ld c, 0
	jr nz, lr_draw
	inc c
lr_draw:
	ld a, (LV_ROW)
	inc a
	ld b, a
	ld hl, LV_TEXT
	ld de, LV_MARK
	jp far_ui_list_row

; The glyph of each stored value: unchecked, checked, failed and resting on something not checked.
list_marks:
	.db UI_FONT_CH_3F, UI_FONT_CH_2713, UI_FONT_CH_2717, UI_FONT_CH_7E
#endif
