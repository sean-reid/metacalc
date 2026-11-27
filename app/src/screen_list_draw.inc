; The list screen's draw (screen_list.inc has the kind's header): the title with the stage's name cut to
; the room checked/total leaves, the rows from the top, the preview and the keys. Strings built here are
; in APP_TEXT and the constants are staged from list_block. Every routine here destroys AF BC DE HL
; and IX, since far_ui_text_width leaves AF BC DE and HL as its result and every far call destroys IX.
; With a find typed (LV_FIND, a copy of frame bytes 7 to 15) the title's right side is the prefix from
; list_find_right in place of the counts: the characters whole when they fit LIST_FIND_ROOM pixels, else an
; ellipsis and the longest end of them that fits, and after a miss LIST_FIND_ROOM_MISS pixels, a space and
; the cross, as the model's tail does.

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
	inc hl
	ld a, (hl)
	ld (LV_KIND), a
	inc hl
	ld de, LV_FIND
	ld bc, LIST_FIND_MAX + 1
	ldir
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
ld_none:
	ld hl, STG_LIST_NONE
	ld b, 3
	call far_ui_text_centered
	jr ld_preview
ld_rows:
	ld a, (LV_KIND)
	or a
	jr z, ld_first
	ld hl, (LV_TOP)
	call lf_next
	ld de, (LV_TOP)
	or a
	sbc hl, de
	jr nz, ld_none
ld_first:
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
	ld a, (LV_KIND)
	or a
	call nz, lf_next
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
; AF BC DE HL and IX, C being the result.
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

; In: HL = an assertion number, DE = where to write LIST_LABEL_MAX + 1 bytes. Out: A = 0 and the first
; LIST_LABEL_MAX characters of its label as glyph codes ended by 0, a character with no glyph being the
; question mark, or the reader's status with nothing written. Reads through FM_SCRATCH. Destroys AF
; BC DE HL and IX.
list_label_text:
	push de
	ld de, FM_SCRATCH
	call far_lib_label
	pop de
	or a
	ret nz
	ld a, b
	cp LIST_LABEL_MAX + 1
	jr c, ll_count
	ld a, LIST_LABEL_MAX
ll_count:
	ld b, a
	ld hl, FM_SCRATCH
ll_glyph:
	ld a, b
	or a
	jr z, ll_cut
	push bc
	ld a, (hl)
	inc hl
	push hl
	sub 20h
	cp 95
	jr c, ll_known
	ld a, 3Fh - 20h
ll_known:
	ld c, a
	ld b, 0
	ld hl, checking_glyphs
	add hl, bc
	ld a, (hl)
	ld (de), a
	inc de
	pop hl
	pop bc
	dec b
	jr ll_glyph
ll_cut:
	xor a
	ld (de), a
	ret

; Draws the title bar: the stage's name, cut to the room the counts leave, and "checked/total" with the
; model's "flat " in front when LV_FLAT is 1.
list_title:
	ld a, (LV_FIND)
	and LIST_FIND_LEN
	jr z, lt_figures
	call list_find_right
	jr lt_stage
lt_figures:
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
lt_stage:
	ld a, (LV_STAGE)
	call stage_strings
	ld de, LV_NAME
	call app_strcpy
	ld hl, LV_RIGHT
	call list_room
	ld a, (LV_KIND)
	or a
	jr z, lt_name
	push bc
	call list_tail
	push hl
	call far_ui_text_width
	ld a, l
	pop hl
	pop bc
	ld b, a
	ld a, c
	sub b
	jr nc, lt_room
	xor a
lt_room:
	ld c, a
lt_name:
	ld hl, LV_NAME
	ld de, LV_CUT
	ld b, LIST_NAME_SIZE
	call far_ui_truncate
	ld a, (LV_KIND)
	or a
	jr z, lt_draw
	ld hl, LV_CUT
lt_end:
	ld a, (hl)
	or a
	jr z, lt_cut
	inc hl
	jr lt_end
lt_cut:
	ex de, hl
	call list_tail
	call app_strcpy
lt_draw:
	ld hl, LV_CUT
	ld de, LV_RIGHT
	jp far_ui_title

; Builds LV_RIGHT, the typed prefix as the model's title shows it: the characters whole when they fit
; 50 pixels, 41 after a miss, else an ellipsis and the longest end of them that fits, and after a miss a
; space and the cross. In: LV_FIND the frame's bytes 7 to 15.
list_find_right:
	ld a, (LV_FIND)
	and LIST_FIND_LEN
	ld b, a
	ld hl, LV_FIND + 1
	ld de, LV_RIGHT + 1
	call ll_glyph
	ld hl, LV_RIGHT + 1
	call lfr_fits
	jr nc, lfr_copy
	ld hl, LV_RIGHT
lfr_cut:
	ld (hl), UI_FONT_CH_2026
	call lfr_fits
	jr nc, lfr_copy
	inc hl
	jr lfr_cut
lfr_copy:
	ld de, LV_RIGHT
	call app_strcpy
	dec de
	ld a, (LV_FIND)
	or a
	ret p
	ld a, UI_FONT_CH_20
	ld (de), a
	inc de
	ld a, UI_FONT_CH_2717
	ld (de), a
	inc de
	xor a
	ld (de), a
	ret

; In: HL = a string of glyph codes. Out: carry set when it is wider than the room the prefix has.
; HL is kept.
lfr_fits:
	push hl
	call far_ui_text_width
	ld a, (LV_FIND)
	rla
	ld a, LIST_FIND_ROOM
	jr nc, lfr_room
	ld a, LIST_FIND_ROOM_MISS
lfr_room:
	cp l
	pop hl
	ret

; Out: HL = the string after the stage's name for the kind in LV_KIND, which is not All.
list_tail:
	ld hl, STG_LIST_THM
	ld a, (LV_KIND)
	cp LISTK_THEOREMS
	ret z
	ld hl, STG_LIST_AX
	cp LISTK_AXIOMS
	ret z
	ld hl, STG_LIST_UNCH
	ret

; Draws the row LV_ROW + 1 for the assertion LV_AT: its label cut to the room its mark leaves, the mark
; and the cursor's inversion.
list_row:
	ld hl, (LV_AT)
	ld de, LV_LABEL
	call list_label_text
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
	ld b, LIST_LABEL_MAX + 2
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
