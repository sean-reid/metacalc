; The theorem screen's layout: the rows each statement takes, where each starts, and the view that follows
; the cursor (screen_theorem.inc has the kind's header).

#ifdef LIBRARY
; Derives TV_INDENT from TV_NHYPS: the width of "N." and 2, 0 for no hypothesis.
theorem_indent:
	ld a, (TV_NHYPS)
	or a
	jr z, ti_set
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
	call far_ui_text_width
	ld a, l
	add a, 2
ti_set:
	ld (TV_INDENT), a
	ret

; In: A = a statement. Out: D = the pixels it is laid out at. Destroys AF E HL.
theorem_width:
	ld d, THM_WIDTH
	ld hl, TV_NHYPS
	cp (hl)
	ret z
	ld a, (TV_INDENT)
	ld e, a
	ld a, d
	sub e
	ld d, a
	ret

; In: A = a statement. Out: A = fm_select's status, NZ when not 0. Destroys all.
theorem_select:
	ld e, a
	ld hl, TV_NHYPS
	cp (hl)
	jr nz, ts_call
	ld e, 0FFh
ts_call:
	call far_fm_select
	or a
	ret

; Derives TV_STARTS and TV_TOTAL from the line counts fm holds, leaving the conclusion selected.
; Out: A = a status, NZ when not 0. Destroys all.
theorem_place:
	ld hl, 0
	ld (TV_ROW), hl
	xor a
	ld (TV_I), a
tp_next:
	ld a, (TV_I)
	call theorem_select
	ret nz
	ld a, (TV_I)
	call theorem_width
	call far_fm_rows
	or a
	ret nz
	ld a, (TV_I)
	add a, a
	ld e, a
	ld d, 0
	ld hl, TV_STARTS
	add hl, de
	ld de, (TV_ROW)
	ld (hl), e
	inc hl
	ld (hl), d
	ld l, b
	ld h, 0
	add hl, de
	ld (TV_ROW), hl
	ld hl, TV_I
	inc (hl)
	ld a, (TV_NHYPS)
	cp (hl)
	jr nc, tp_next
	ld hl, (TV_ROW)
	ld (TV_TOTAL), hl
	ld a, (TV_NHYPS)
	inc a
	add a, a
	ld e, a
	ld d, 0
	ld hl, TV_STARTS
	add hl, de
	ld de, (TV_TOTAL)
	ld (hl), e
	inc hl
	ld (hl), d
	xor a
	ret

; Counts the rows of the selected statement TV_S again after a fold and moves the first row of every
; statement after it, and TV_TOTAL, by the difference. Out: A = a status, NZ when not 0. Destroys all.
theorem_recount:
	ld a, (TV_S)
	call theorem_width
	call far_fm_rows
	or a
	ret nz
	ld a, (TV_S)
	add a, a
	ld e, a
	ld d, 0
	ld hl, TV_STARTS
	add hl, de
	ld e, (hl)
	inc hl
	ld d, (hl)
	inc hl
	push hl
	ld c, (hl)
	inc hl
	ld h, (hl)
	ld l, c
	or a
	sbc hl, de
	ld e, b
	ld d, 0
	ex de, hl
	or a
	sbc hl, de
	ex de, hl
	ld a, (TV_NHYPS)
	inc a
	ld hl, TV_S
	sub (hl)
	ld b, a
	pop hl
tr_shift:
	ld a, (hl)
	add a, e
	ld (hl), a
	inc hl
	ld a, (hl)
	adc a, d
	ld (hl), a
	inc hl
	djnz tr_shift
	dec hl
	ld d, (hl)
	dec hl
	ld e, (hl)
	ld (TV_TOTAL), de
	xor a
	ret

; In: HL = a first row. Out: HL = the first row, at most the rows less six, or 0. Destroys AF DE.
theorem_clamp:
	push hl
	ld hl, (TV_TOTAL)
	ld de, -THM_ROWS
	add hl, de
	jr c, tc_max
	ld hl, 0
tc_max:
	pop de
	push de
	push hl
	or a
	sbc hl, de
	pop hl
	pop de
	ret c
	ex de, hl
	ret

; Out: HL = the first row the model's followed() gives for the cursor and TV_SCROLL, A = a status, NZ when
; not 0. Destroys all.
theorem_followed:
	ld a, (TV_S)
	add a, a
	ld e, a
	ld d, 0
	ld hl, TV_STARTS
	add hl, de
	ld e, (hl)
	inc hl
	ld d, (hl)
	ld a, (TV_N)
	or a
	jr nz, tf_node
	inc hl
	ld c, (hl)
	inc hl
	ld b, (hl)
	ld (TV_TOP), de
	ld (TV_BOT), bc
	jr tf_view
tf_node:
	ld (TV_TOP), de
	ld a, (TV_S)
	call theorem_select
	ret nz
	ld a, (TV_S)
	call theorem_width
	ld a, (TV_N)
	ld e, a
	call far_fm_span
	or a
	ret nz
	ld de, (TV_TOP)
	ld l, b
	ld h, 0
	add hl, de
	ld (TV_TOP), hl
	ld l, c
	ld h, 0
	add hl, de
	ld (TV_BOT), hl
tf_view:
	ld de, (TV_SCROLL)
	ld hl, (TV_TOP)
	or a
	sbc hl, de
	ld hl, (TV_TOP)
	jr c, tf_clamp
	ld hl, THM_ROWS
	add hl, de
	ex de, hl
	ld hl, (TV_BOT)
	or a
	sbc hl, de
	ld hl, (TV_SCROLL)
	jr c, tf_clamp
	jr z, tf_clamp
	ld hl, (TV_BOT)
	ld de, -THM_ROWS
	add hl, de
	ld de, (TV_TOP)
	push hl
	or a
	sbc hl, de
	pop hl
	jr c, tf_clamp
	ex de, hl
tf_clamp:
	call theorem_clamp
	xor a
	ret

; Moves the view to the cursor when TV_DOFOLLOW says so, then sets TV_FOLLOWING from whether the view is
; where follow would put it. Out: A = a status, NZ when not 0. Destroys all.
theorem_finish:
	call theorem_followed
	ret nz
	ld a, (TV_DOFOLLOW)
	or a
	jr z, tn_keep
	ld (TV_SCROLL), hl
tn_keep:
	ld de, (TV_SCROLL)
	or a
	sbc hl, de
	ld a, 0
	jr nz, tn_set
	inc a
tn_set:
	ld (TV_FOLLOWING), a
	xor a
	ret

; Writes TV_S to TV_FOLLOWING into the frame's bytes 4 to 8. Out: A = 0. Destroys AF BC DE HL.
theorem_store:
	ld hl, (TV_FRAME)
	ld de, 4
	add hl, de
	ex de, hl
	ld hl, TV_S
	ld bc, 5
	ldir
	xor a
	ret
#endif
