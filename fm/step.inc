; fm_step and fm_span: where a node is in the tree and on which lines it is drawn.

; in: E = a node, D = 0 the parent, 1 the first child, 2 the next sibling, 3 the previous sibling
step_body:
	call fm_need
	jp nz,fm_fail
	ld a,d
	cp 4
	ld a,FM_ERR_BAD
	jp nc,fm_fail
	ld a,d
	ld (fv_fm),a
	ld a,e
	ld (fv_fn),a
	call fo_node
	jp nz,fm_fail
	ld a,(fv_fn)
	ld de,2*FM_PLANE
	call fo_row
	ld c,(hl)
	ld a,(fv_fm)
	or a
	jr z,st_parent
	dec a
	jr z,st_child
	dec a
	jr z,st_next
	jr st_back
st_parent:
	ld a,c
	or a
	jr z,st_none
	dec c
st_back:
	ld a,(fv_fn)
	ld b,a
st_prev:
	ld a,b
	or a
	jr z,st_none
	dec b
	dec hl
	ld a,(hl)
	cp c
	jr z,st_found
	jr nc,st_prev
st_none:
	ld b,$FF
st_found:
	xor a
	jp fm_leave
st_child:
	ld a,(fv_fn)
	inc a
	ld b,a
	ld hl,fv_tcnt
	cp (hl)
	jr nc,st_none
	ld a,(fv_fn)
	ld de,2*FM_PLANE
	call fo_row
	inc hl
	ld a,(hl)
	cp c
	jr c,st_none
	jr z,st_none
	jr st_found
st_next:
	ld a,(fv_fn)
	ld b,a
st_after:
	inc b
	ld a,(fv_tcnt)
	cp b
	jr z,st_none
	inc hl
	ld a,(hl)
	cp c
	jr z,st_found
	jr nc,st_after
	jr st_none

; in: E = a node, D = the width in pixels. out: B = the first line the node is drawn on, C = one past
; the last. The tokens are placed as ly_scan places them.
span_body:
	call fm_need
	jp nz,fm_fail
	ld a,d
	ld (fv_limit),a
	ld a,e
	ld (fv_fn),a
	call fo_node
	jp nz,fm_fail
	call fm_hdr7
	bit FM_CLIPPED,(hl)
	ld a,FM_ERR_CLIPPED
	jp nz,fm_fail
	call fo_fresh
	ld a,(fv_curn)
	push af
	ld a,(fv_fn)
	ld (fv_curn),a
	call cu_prepare
	pop af
	ld (fv_curn),a
	ld a,(fv_count)
	ld b,a
	ld ix,(fv_tok)
	ld c,0
	ld d,$FF
sp_line:
	inc d
	ld a,(fv_limit)
	sub (ix+1)
	jr nc,sp_room
	xor a
sp_room:
	ld e,a
sp_token:
	ld a,(fv_cf)
	cp c
	jr nz,sp_last
	ld a,d
	ld (fv_xs),a
sp_last:
	ld a,(fv_cl)
	cp c
	jr nz,sp_step
	ld a,d
	ld (fv_xe),a
sp_step:
	inc c
	inc ix
	inc ix
	inc ix
	inc ix
	dec b
	jr z,sp_done
	ld a,(ix+1)
	add a,(ix+2)
	jr c,sp_line
	ld l,a
	ld a,e
	sub l
	jr c,sp_line
	ld e,a
	jr sp_token
sp_done:
	ld a,(fv_xe)
	inc a
	ld c,a
	ld a,(fv_xs)
	ld b,a
	xor a
	jp fm_leave
