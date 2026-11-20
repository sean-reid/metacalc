; The cursor: fm_cursor stores the node, fm_draw inverts the pieces of it on each line it draws.

; in: E = a node, FFh for none. Stores it when the table is built and has the node.
cursor_body:
	call fm_need
	jp nz,fm_fail
	ld a,e
	ld (fv_fn),a
	inc a
	jr z,cb_store
	call fo_node
	jp nz,fm_fail
cb_store:
	ld a,(fv_fn)
	ld (fv_curn),a
	xor a
	ld b,a
	jp fm_leave

; Sets fv_cc when the cursor node is in the selected statement's table: fv_cf and fv_cl are then
; the first and last token of the node drawn in its place, the node itself or the outermost
; folded node it is hidden inside. Destroys all but IX and IY.
cu_prepare:
	call fo_table
	ret z
	ld a,(fv_curn)
	ld hl,fv_tcnt
	cp (hl)
	ret nc
	call fo_effective
	ld de,0
	call fo_row
	ld a,(hl)
	ld (fv_cf),a
	ld de,FM_PLANE
	add hl,de
	ld a,(hl)
	ld (fv_cl),a
	ld a,1
	ld (fv_cc),a
	ret

; fm_lay calls cu_start, ly_draw and cu_end for a line it draws with the cursor set. The line is the
; tokens fv_cs to fv_ce - 1, from the record at fv_cptr, with fv_room columns left; cu_end inverts
; the cursor's pieces on it. A line the node holds whole, which is every line of a node that runs
; to the end of the statement, is inverted from column 0 to the width less the columns left, the
; way cu_rect would.
cu_start:
	exx
	ld (fv_cptr),hl
	ld a,(fv_count)
	sub b
	ld (fv_cs),a
	exx
	ret

cu_end:
	exx
	ld a,(fv_count)
	sub b
	ld d,a
	ld e,c
	push de
	exx
	pop de
	ld bc,(fv_cf)
	ld a,(fv_cs)
	ld h,a
	ld a,b
	cp h
	jr c,cl_off
	ld a,c
	cp d
	ret nc
	ld a,b
	cp d
	jr nc,cl_keep
	xor a
	ld (fv_cc),a
cl_keep:
	ld a,h
	cp c
	jr c,cl_part
	ld a,d
	dec a
	cp b
	jr c,cl_whole
	jr nz,cl_part
cl_whole:
	ld hl,(fv_cptr)
	inc hl
	ld a,(fv_limit)
	cp (hl)
	jr c,cl_part
	sub e
	add a,2
	jr nc,cw_w
	ld a,$FF
cw_w:
	ld d,a
	ld a,(fv_rowb)
	cp UI_TEXT_ROWS
	ret nc
	add a,a
	add a,a
	add a,a
	ld c,a
	ld a,(fv_x0)
	or a
	jr z,cw_x
	dec a
cw_x:
	ld b,a
	ld e,8
	jp ui_invert
cl_part:
	ld a,d
	ld (fv_ce),a
	ld a,e
	ld (fv_room),a
	jp cu_rect
cl_off:
	xor a
	ld (fv_cc),a
	ret

; Inverts the part of the line that belongs to the node: from the x of its first piece on the line
; to the end of its last, padded by one column each side and clipped to the screen. The start walks
; the tokens before the first piece. The end is where the line ends less the tokens after the last
; piece, or a walk over the pieces when the first token is wider than the width, which leaves no
; column and makes the line's end more than the width less the columns left.
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
	ld hl,(fv_cptr)
	inc hl
	ld e,(hl)
	ld d,0
	inc hl
	inc b
	dec b
	jr z,cr_at
cr_skip:
	call cu_step
	djnz cr_skip
cr_at:
	ld a,d
	ld (fv_xs),a
	push hl
	ld hl,(fv_cptr)
	inc hl
	ld a,(fv_limit)
	cp (hl)
	pop hl
	jr c,cr_span
	ld a,(fv_ce)
	ld hl,fv_cb
	sub (hl)
	dec a
	ld b,a
	ld c,0
	jr z,cr_total
	ld a,(hl)
	inc a
	ld hl,fv_cs
	sub (hl)
	ld l,a
	ld h,0
	add hl,hl
	add hl,hl
	ld de,(fv_cptr)
	add hl,de
cr_tail:
	inc hl
	ld a,(hl)
	inc hl
	add a,(hl)
	add a,c
	ld c,a
	inc hl
	inc hl
	djnz cr_tail
cr_total:
	ld a,(fv_limit)
	ld hl,fv_room
	sub (hl)
	sub c
	jr cr_end
cr_span:
	push hl
	ld hl,fv_ca
	ld a,(fv_cb)
	sub (hl)
	pop hl
	ld b,a
	jr z,cr_last
cr_walk:
	call cu_step
	djnz cr_walk
cr_last:
	ld a,e
cr_end:
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
