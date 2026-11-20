; fm_open's second half: copies the shapes section into FM_SECOND and checks the copy, with the
; syntax axioms' arities from one lib_arities call in FM_PILE. A library with neither trees nor
; shapes opens as it is; every statement of it is one node.

op_shapes:
	call lib_syntax_count
	jr z,os_trees
	cp LIB_ERR_NONE
	jr z,os_plain
	call fm_reader_status
	jp fm_fail
os_plain:
	call lib_shapes
	cp LIB_ERR_NONE
	jp z,op_done
	jp op_bad
os_trees:
	ld (fv_ns),hl
	call lib_shapes
	jr z,os_got
	cp LIB_ERR_NONE
	jp z,op_bad
	call fm_reader_status
	jp fm_fail
os_got:
	ld (fv_sk),hl
	ld a,e
	ld (fv_sk+2),a
	ld (fv_sk+3),bc
	ld a,1
	ld (fv_sk+5),a
	ld (fv_src),hl
	ld a,e
	ld (fv_src+2),a
	ld (fv_shl),bc
	ld hl,FM_SECOND_SIZE
	or a
	sbc hl,bc
	ld a,FM_ERR_TOO_BIG
	jp c,fm_fail
	ld (fv_left),bc
	ld hl,FM_SECOND
	ld (fv_dst),hl
	call fm_copy

; The count is the reader's, the offset table fits, and every item lies after the table and inside
; the copy, has 1 to 255 parts, and holds each child of its axiom's arity once and no other.
	ld hl,(FM_SECOND)
	ld de,(fv_ns)
	or a
	sbc hl,de
	jp nz,op_bad
	ld hl,(fv_ns)
	inc hl
	inc hl
	add hl,hl
	ld (fv_te),hl
	ld de,(fv_shl)
	or a
	sbc hl,de
	jr z,os_start
	jp nc,op_bad
os_start:
	ld hl,0
	ld (fv_si),hl
	ld de,FM_PILE
	ld bc,FM_PILE_SIZE
	call lib_arities
	ld (fv_left),hl
	ld (fv_byte),a
os_item:
	ld hl,(fv_si)
	ld de,(fv_ns)
	or a
	sbc hl,de
	jp z,op_done
	ld hl,(fv_si)
	add hl,hl
	ld de,FM_SECOND+2
	add hl,de
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld c,(hl)
	inc hl
	ld b,(hl)
	ld (fv_so),de
	ld h,b
	ld l,c
	ld de,(fv_shl)
	or a
	sbc hl,de
	jr z,os_inside
	jp nc,op_bad
os_inside:
	ld hl,(fv_so)
	ld de,(fv_te)
	or a
	sbc hl,de
	jp c,op_bad
	ld hl,(fv_so)
	ld a,c
	sub l
	ld e,a
	ld a,b
	sbc a,h
	jp c,op_bad
	jp nz,op_bad
	ld a,e
	or a
	jp z,op_bad
	ld (fv_shn),a
	ld hl,(fv_si)
	ld de,(fv_left)
	or a
	sbc hl,de
	jr c,os_arity
	ld a,(fv_byte)
	call fm_reader_status
	jp fm_fail
os_arity:
	ld hl,(fv_si)
	ld de,FM_PILE
	add hl,de
	ld b,(hl)
	ld a,b
	ld (fv_ar),a
	or a
	jr z,os_scan
	ld hl,FM_SCRATCH
os_clear:
	ld (hl),0
	inc hl
	djnz os_clear
os_scan:
	ld hl,(fv_so)
	ld de,FM_SECOND
	add hl,de
	ld a,(fv_shn)
	ld b,a
	ld c,0
os_part:
	ld a,(hl)
	inc hl
	or a
	jr z,os_part_next
	ld e,a
	ld a,(fv_ar)
	cp e
	jp c,op_bad
	push hl
	dec e
	ld d,0
	ld hl,FM_SCRATCH
	add hl,de
	ld a,(hl)
	or a
	jp nz,op_bad
	inc (hl)
	pop hl
	inc c
os_part_next:
	djnz os_part
	ld a,(fv_ar)
	cp c
	jp nz,op_bad
	ld hl,(fv_si)
	inc hl
	ld (fv_si),hl
	jp os_item
