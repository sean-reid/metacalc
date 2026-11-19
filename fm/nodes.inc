; fm_nodes: the node table of the loaded statement, from its stored tree and the shapes copy.
;
; The tree's nodes are numbered in preorder as stored, children in the order of the axiom's
; floating hypotheses, which is not always reading order. Pass 1 reads the tree once, with an
; explicit stack in FM_PILE of (node, children still to come) pairs, and leaves for every tree node
; its token count (FM_TLEN), its table node count (FM_TCNT) and its size in tree nodes (FM_TSIZE).
; Pass 2 reads it again in the same order. A node's first token and table row are in FM_TLEN and
; FM_TCNT by then, put there by its parent, which walks its shape in reading order and gives each
; child the tokens and rows that come before it. A node whose only part is a child has no row of
; its own: the child takes its place.

nodes_body:
	call fm_need
	jp nz,fm_fail
	call fm_same_library
	jp nz,fm_fail
	ld a,(fv_count)
	or a
	ld a,FM_ERR_INDEX
	jp z,fm_fail
	ld hl,(fv_hdr)
	inc hl
	inc hl
	inc hl
	ld a,(hl)
	or a
	jr z,nb_build
	ld b,a
	xor a
	jp fm_leave
nb_build:
	ld hl,(fv_hdr)
	ld de,6
	add hl,de
	ld b,(hl)
	ld hl,(fv_cn)
	ld de,FM_SCRATCH
	call lib_tree
	jr z,nb_tree
	cp LIB_ERR_NONE
	jr z,nb_flat
	call fm_reader_status
	jp fm_fail
nb_flat:
	ld a,1
	call nb_alloc
	jp nz,fm_fail
	ld hl,(fv_np)
	ld bc,FM_PLANE
	ld (hl),0
	add hl,bc
	ld a,(fv_count)
	dec a
	ld (hl),a
	add hl,bc
	ld (hl),0
	ld a,1
	jp nb_done
nb_tree:
	ld (fv_tl),bc
	ld (fv_tleft),bc
	ld (fv_ts),hl
	ld a,e
	ld (fv_ts+2),a
	ld a,b
	or a
	ld a,c
	jr z,nb_held
	ld a,255
nb_held:
	ld (fv_tc),a
	ld hl,FM_SCRATCH
	ld (fv_tp),hl
	xor a
	ld (fv_ti),a
	ld hl,FM_PILE
	ld (fv_pile),hl

; Pass 1.
p1_next:
	call tn_next
	jp nz,fm_fail
	ld a,(fv_ti)
	inc a
	ld a,FM_ERR_LIMIT
	jp z,fm_fail
	ld a,b
	or a
	jr z,p1_axiom
	ld a,(fv_ti)
	ld l,a
	ld h,FM_TLEN>>8
	ld (hl),1
	inc h
	ld (hl),1
	inc h
	ld (hl),1
	ld hl,fv_ti
	inc (hl)
	jr p1_up
p1_axiom:
	call sh_find
	call sh_scan
	ld a,(fv_ti)
	ld l,a
	ld h,FM_TLEN>>8
	ld (hl),c
	inc h
	ld e,1
	ld a,c
	or a
	jr nz,p1_own
	ld a,d
	cp 1
	jr nz,p1_own
	ld e,0
p1_own:
	ld (hl),e
	inc h
	ld (hl),1
	ld hl,fv_ti
	ld e,(hl)
	inc (hl)
	ld a,d
	or a
	jr z,p1_leaf
	ld hl,(fv_pile)
	ld (hl),e
	inc hl
	ld (hl),d
	inc hl
	ld (fv_pile),hl
	jp p1_next
p1_leaf:
	ld a,e
	jr p1_done
p1_up:
	ld hl,fv_ti
	ld a,(hl)
	dec a
p1_done:
	ld e,a
p1_climb:
	ld hl,(fv_pile)
	ld bc,FM_PILE
	or a
	sbc hl,bc
	jr z,p1_root
	ld hl,(fv_pile)
	dec hl
	ld a,(hl)
	dec a
	ld (hl),a
	ld (fv_tmp),a
	dec hl
	ld d,(hl)
	ld h,FM_TLEN>>8
	ld l,e
	ld c,(hl)
	ld l,d
	ld a,(hl)
	add a,c
	ld a,FM_ERR_TREE
	jp c,fm_fail
	ld a,(hl)
	add a,c
	ld (hl),a
	inc h
	ld l,e
	ld c,(hl)
	ld l,d
	ld a,(hl)
	add a,c
	ld (hl),a
	inc h
	ld l,e
	ld c,(hl)
	ld l,d
	ld a,(hl)
	add a,c
	ld (hl),a
	ld a,(fv_tmp)
	or a
	jp nz,p1_next
	ld hl,(fv_pile)
	dec hl
	dec hl
	ld (fv_pile),hl
	ld e,d
	jr p1_climb
p1_root:
	ld hl,(fv_tleft)
	ld a,h
	or l
	ld a,FM_ERR_TREE
	jp nz,fm_fail
	ld a,(FM_TLEN)
	inc a
	ld hl,fv_count
	cp (hl)
	ld a,FM_ERR_TREE
	jp nz,fm_fail

; Pass 2. The root starts at token 1, in row 0; table row 0 spans the lead too.
	ld a,(FM_TCNT)
	ld (fv_total),a
	call nb_alloc
	jp nz,fm_fail
	ld hl,(fv_np)
	ld bc,FM_PLANE
	add hl,bc
	ld a,(FM_TLEN)
	ld (hl),a
	add hl,bc
	ld (hl),0
	ld a,1
	ld (FM_TLEN),a
	xor a
	ld (FM_TCNT),a
	ld (fv_cur),a
	ld (fv_tc),a
	ld hl,(fv_tl)
	ld (fv_tleft),hl
p2_loop:
	call tn_next
	jp nz,fm_fail
	ld a,(fv_cur)
	ld l,a
	ld h,FM_TLEN>>8
	ld c,(hl)
	ld a,b
	or a
	jr z,p2_axiom
	push de
	ld l,c
	ld h,0
	add hl,hl
	add hl,hl
	ld de,(fv_tok)
	add hl,de
	pop de
	ld a,(hl)
	cp e
	ld a,FM_ERR_TREE
	jp nz,fm_fail
	jp p2_next
p2_axiom:
	call sh_find
	ld (fv_shp),hl
	ld a,b
	ld (fv_shn),a
	call sh_scan
	ld e,1
	ld a,c
	or a
	jr nz,p2_own
	ld a,d
	cp 1
	jr nz,p2_own
	ld e,0
p2_own:
	ld a,(fv_cur)
	ld l,a
	ld h,FM_TLEN>>8
	ld a,(hl)
	ld (fv_pos),a
	inc h
	ld a,(hl)
	ld c,a
	add a,e
	ld (fv_nrow),a
	ld b,0
	push de
	ld hl,(fv_np)
	ld de,2*FM_PLANE
	add hl,de
	add hl,bc
	ld a,(hl)
	pop de
	add a,e
	ld (fv_dep),a
p2_part:
	ld hl,(fv_shp)
	ld a,(hl)
	inc hl
	ld (fv_shp),hl
	or a
	jr nz,p2_slot
	ld hl,fv_pos
	inc (hl)
	jr p2_more
p2_slot:
	dec a
	ld b,a
	ld a,(fv_cur)
	inc a
	ld c,a
	ld h,FM_TSIZE>>8
p2_hop:
	ld a,b
	or a
	jr z,p2_found
	ld l,c
	ld a,(hl)
	add a,c
	ld c,a
	dec b
	jr p2_hop
p2_found:
	ld h,FM_TLEN>>8
	ld l,c
	ld d,(hl)
	ld a,(fv_pos)
	ld (hl),a
	ld e,a
	inc h
	ld b,(hl)
	ld a,(fv_nrow)
	ld (hl),a
	push bc
	ld c,a
	ld b,0
	ld hl,(fv_np)
	add hl,bc
	ld (hl),e
	ld bc,FM_PLANE
	add hl,bc
	ld a,e
	add a,d
	dec a
	ld (hl),a
	add hl,bc
	ld a,(fv_dep)
	ld (hl),a
	pop bc
	ld a,e
	add a,d
	ld (fv_pos),a
	ld a,(fv_nrow)
	add a,b
	ld (fv_nrow),a
p2_more:
	ld hl,fv_shn
	dec (hl)
	jr nz,p2_part
p2_next:
	ld hl,fv_cur
	inc (hl)
	ld a,(hl)
	ld hl,fv_ti
	cp (hl)
	jp nz,p2_loop
	ld hl,(fv_np)
	ld (hl),0
	ld a,(fv_total)
	jp nb_done

; in: A = the node count of the table about to be built. out: Z with fv_np at its first row in the
; FIRST plane, or NZ with A = FM_ERR_LIMIT when the assertion's tables would pass FM_MAX_NODES.
nb_alloc:
	ld c,a
	ld b,0
	ld hl,(fv_nused)
	add hl,bc
	ld de,FM_MAX_NODES+1
	or a
	sbc hl,de
	jr c,na_fit
	ld a,FM_ERR_LIMIT
	or a
	ret
na_fit:
	ld hl,(fv_nused)
	ld de,FM_FIRST
	add hl,de
	ld (fv_np),hl
	xor a
	ret

; in: A = the node count of the table just built at fv_np. Marks the statement built.
nb_done:
	ld hl,(fv_hdr)
	inc hl
	inc hl
	inc hl
	ld (hl),a
	inc hl
	ld de,(fv_nused)
	ld (hl),e
	inc hl
	ld (hl),d
	ld hl,(fv_nused)
	ld c,a
	ld b,0
	add hl,bc
	ld (fv_nused),hl
	ld b,a
	xor a
	jp fm_leave

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
