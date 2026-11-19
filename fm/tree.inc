; The tree reader and the shape lookups fm_nodes uses.

; out: Z with B = 0 for a syntax index and 1 for a variable, DE = the index or the variable's
; symbol id, for the next node of the tree; NZ with A = a status. Reads FM_SCRATCH again when it
; holds fewer than 2 bytes and more of the tree is left.
tn_next:
	ld a,(fv_tc)
	cp 2
	jr nc,tn_ready
	ld e,a
	ld d,0
	ld hl,(fv_tleft)
	or a
	sbc hl,de
	jr z,tn_ready
	call tn_refill
	ret nz
tn_ready:
	ld a,(fv_tc)
	or a
	jr z,tn_bad
	ld c,a
	ld hl,(fv_tp)
	call lib_node
	jr nz,tn_bad
	ld (fv_tp),hl
	push de
	push bc
	ld a,(fv_tc)
	sub c
	ld e,a
	ld d,0
	ld a,c
	ld (fv_tc),a
	ld hl,(fv_tleft)
	or a
	sbc hl,de
	ld (fv_tleft),hl
	pop bc
	pop de
	xor a
	ret
tn_bad:
	ld a,FM_ERR_TREE
	or a
	ret

; Reads the next 255 bytes of the tree, or what is left of it, into FM_SCRATCH.
tn_refill:
	ld hl,(fv_tl)
	ld de,(fv_tleft)
	or a
	sbc hl,de
	ld de,(fv_ts)
	add hl,de
	ld a,(fv_ts+2)
	adc a,0
	ld (fv_src),hl
	ld (fv_src+2),a
	ld bc,(fv_tleft)
	ld a,b
	or a
	jr z,tr_held
	ld bc,255
tr_held:
	ld a,c
	ld (fv_tc),a
	ld hl,FM_SCRATCH
	ld (fv_tp),hl
	ld de,FM_SCRATCH
	ld hl,(fv_src)
	ld a,(fv_src+2)
	call lib_read
	ret z
	call fm_reader_status
	or a
	ret

; in: DE = a syntax index. out: HL = its shape in the copy, B = its length in bytes.
sh_find:
	ld hl,FM_SECOND+2
	add hl,de
	add hl,de
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld c,(hl)
	inc hl
	ld b,(hl)
	ld hl,FM_SECOND
	add hl,de
	push hl
	ld h,b
	ld l,c
	or a
	sbc hl,de
	ld b,l
	pop hl
	ret

; in: HL = a shape, B = its length. out: C = its constants, D = its children.
sh_scan:
	ld c,0
	ld d,0
sc_loop:
	ld a,(hl)
	inc hl
	or a
	jr nz,sc_child
	inc c
	djnz sc_loop
	ret
sc_child:
	inc d
	djnz sc_loop
	ret
