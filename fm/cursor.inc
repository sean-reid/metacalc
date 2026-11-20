; fm_cursor: inverts the runs of a node as the model's cursor does, over what fm_draw drew.

; in: A = a node, HL, D, E, B and C as fm_draw takes them. Inverts the node's runs, the blank
; column on each side of them included, on the lines inside the window. The A of the call is in A' here.
cursor_body:
	ex af,af'
	ld (fv_fn),a
	call fm_need
	jp nz,fm_fail
	ld a,d
	ld (fv_limit),a
	ld a,e
	ld (fv_x0),a
	ld (fv_row),hl
	ld a,b
	ld (fv_lo),a
	ld a,c
	ld (fv_hi),a
	call fo_node
	jp nz,fm_fail
	call fo_fresh
	ld a,(fv_fn)
	call fo_effective
	ld de,0
	call fo_row
	ld a,(hl)
	ld (fv_cf),a
	ld de,FM_PLANE
	add hl,de
	ld a,(hl)
	ld (fv_cl),a
	xor a
	ld (fv_line),a
	exx
	ld a,(fv_count)
	ld b,a
	ld hl,(fv_tok)
	exx
cu_line:
	ld hl,fv_line
	inc (hl)
	call fy_window
	jr c,cu_done
	exx
	ld (fv_cptr),hl
	ld a,(fv_count)
	sub b
	ld (fv_cs),a
	exx
	call ly_scan
	exx
	ld a,(fv_count)
	sub b
	ld (fv_ce),a
	exx
	ld a,(fv_cl)
	ld hl,fv_cs
	cp (hl)
	jr c,cu_done
	ld a,(fv_ce)
	ld hl,fv_cf
	cp (hl)
	jr c,cu_next
	jr z,cu_next
	ld a,(fv_show)
	or a
	call nz,cu_rect
cu_next:
	exx
	ld a,b
	exx
	or a
	jr nz,cu_line
cu_done:
	xor a
	ld b,a
	jp fm_leave

; Inverts the part of the line just scanned that belongs to the node: from the x of its first token
; on the line to the end of its last, padded by one column each side and clipped to the screen.
cu_rect:
	ld a,(fv_cs)
	ld hl,fv_cf
	cp (hl)
	jr nc,cr_lo
	ld a,(hl)
cr_lo:
	ld (fv_ca),a
	ld a,(fv_ce)
	dec a
	ld hl,fv_cl
	cp (hl)
	jr c,cr_hi
	ld a,(hl)
cr_hi:
	ld (fv_cb),a
	ld hl,fv_cs
	ld a,(fv_ca)
	sub (hl)
	ld b,a
	ld hl,fv_ca
	ld a,(fv_cb)
	sub (hl)
	ld (fv_tmp),a
	ld hl,(fv_cptr)
	inc hl
	ld e,(hl)
	ld d,0
	inc hl
	ld a,b
	or a
	jr z,cr_at
cr_skip:
	call cu_step
	djnz cr_skip
cr_at:
	ld a,d
	ld (fv_xs),a
	ld a,(fv_tmp)
	ld b,a
	or a
	jr z,cr_end
cr_span:
	call cu_step
	djnz cr_span
cr_end:
	ld a,e
	ld (fv_xe),a
	ld a,(fv_rowb)
	cp UI_TEXT_ROWS
	ret nc
	add a,a
	add a,a
	add a,a
	ld c,a
	ld a,(fv_x0)
	ld hl,fv_xs
	add a,(hl)
	ret c
	jr z,cr_x
	dec a
cr_x:
	ld b,a
	ld a,(fv_xe)
	sub (hl)
	add a,2
	jr nc,cr_w
	ld a,$FF
cr_w:
	ld d,a
	ld e,8
	jp ui_invert

; Steps from a token to the next on its line. in: HL = the gap of the token, D = its x, E = its end.
; out: the same for the next token. Keeps B.
cu_step:
	inc hl
	inc hl
	inc hl
	ld a,(hl)
	inc hl
	ld c,(hl)
	ld d,a
	ld a,e
	add a,c
	ld e,a
	add a,d
	ld d,e
	ld e,a
	ret
