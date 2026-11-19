; Header, counts and the symbol index.

lo_magic:
	.db "MCL1"
lo_hdr_len:
	.db 20,0,0

lib_open:
	ld (lr_scratch),de
	ld (lr_page),a
	ld hl,0
	ld (lr_open),hl
	ld bc,256
	call lr_chkbuf
	ret nz
	ld a,(lr_page)
	cp $80
	jp nc,lo_bad
	ld hl,20
	ld (lr_length),hl
	xor a
	ld (lr_length+2),a
	ld de,(lr_scratch)
	ld bc,20
	ld hl,0
	call lr_rd
	ret nz

	ld hl,(lr_scratch)
	ld de,lo_magic
	ld b,4
lo_magic_loop:
	ld a,(de)
	cp (hl)
	jp nz,lo_bad
	inc de
	inc hl
	djnz lo_magic_loop
	ld de,4
	add hl,de
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld c,(hl)
	inc hl
	ld a,(hl)
	or a
	jp nz,lo_bad
	ld (lr_tr),de
	ld a,c
	ld (lr_tr+2),a
	inc hl
	ld a,(hl)
	ld (lr_consts),a
	ld c,a
	inc hl
	ld a,(hl)
	add a,c
	jp c,lo_bad
	ld (lr_nsyms),a
	inc hl
	ld e,(hl)
	inc hl
	ld d,(hl)
	ld (lr_count),de
	inc hl
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld c,(hl)
	inc hl
	ld a,(hl)
	or a
	jp nz,lo_bad
	ld (lr_dir),de
	ld a,c
	ld (lr_dir+2),a

	ld a,(lr_dir)
	and 3
	jp nz,lo_bad
	ld hl,(lr_tr)
	ld a,(lr_tr+2)
	ld de,lo_hdr_len
	call lr_cmp24
	jp c,lo_bad
	ld a,(lr_page)
	inc a
	ld b,a
	and 3
	rrca
	rrca
	ld (lr_hdr+1),a
	ld a,b
	rrca
	rrca
	and $3F
	ld (lr_hdr+2),a
	xor a
	ld (lr_hdr),a
	ld hl,(lr_hdr)
	ld a,(lr_hdr+2)
	ld de,lr_tr
	call lr_cmp24
	jp c,lo_bad
	ld hl,(lr_count)
	xor a
	add hl,hl
	rla
	add hl,hl
	rla
	ld de,(lr_dir)
	add hl,de
	ld c,a
	ld a,(lr_dir+2)
	adc a,c
	ld (lr_hdr),hl
	ld (lr_hdr+2),a
	ld hl,(lr_tr)
	ld a,(lr_tr+2)
	ld de,lr_hdr
	call lr_cmp24
	jp c,lo_bad
	ld hl,(lr_tr)
	ld (lr_length),hl
	ld a,(lr_tr+2)
	ld (lr_length+2),a

	ld hl,0
	ld (lr_sr),hl
	xor a
	ld (lr_sid),a
lo_block:
	ld a,(lr_sid)
	ld (lr_sbase),a
	ld hl,lr_nsyms
	cp (hl)
	jp nc,lo_done
	ld hl,(lr_sr)
	ld de,20
	add hl,de
	ld a,0
	adc a,0
	ld (lr_ent),hl
	ld (lr_ent+2),a
	ld de,(lr_ent)
	ld hl,(lr_dir)
	or a
	sbc hl,de
	ld a,(lr_ent+2)
	ld b,a
	ld a,(lr_dir+2)
	sbc a,b
	jp c,lo_bad
	or h
	ld bc,256
	jr nz,lo_take
	ld a,l
	or a
	jp z,lo_bad
	ld c,a
	ld b,0
lo_take:
	push bc
	ld de,(lr_scratch)
	ld hl,(lr_ent)
	ld a,(lr_ent+2)
	call lr_rd
	pop bc
	ret nz
	ld hl,(lr_scratch)
lo_entry:
	ld a,(lr_nsyms)
	ld e,a
	ld a,(lr_sid)
	cp e
	jp nc,lo_done
	ld a,b
	or a
	jr nz,lo_room
	ld a,c
	cp 2
	jr c,lo_more
lo_room:
	inc hl
	ld e,(hl)
	dec hl
	ld a,e
	cp 255
	jp z,lo_bad
	ld d,0
	inc de
	inc de
	push hl
	ld h,b
	ld l,c
	or a
	sbc hl,de
	pop hl
	jr c,lo_more
	push de
	ld a,(lr_consts)
	ld d,a
	ld a,(lr_sid)
	cp d
	ld a,(hl)
	jr nc,lo_variable
	cp $FF
	jr nz,lo_bad_pop
	jr lo_typecode_ok
lo_variable:
	cp d
	jr nc,lo_bad_pop
lo_typecode_ok:
	pop de
	push hl
	push bc
	ld a,(lr_sid)
	ld l,a
	ld h,0
	add hl,hl
	ld bc,LIB_INDEX
	add hl,bc
	ld bc,(lr_sr)
	ld (hl),c
	inc hl
	ld (hl),b
	ld hl,(lr_sr)
	add hl,de
	ld (lr_sr),hl
	pop bc
	pop hl
	add hl,de
	push hl
	ld h,b
	ld l,c
	or a
	sbc hl,de
	ld b,h
	ld c,l
	pop hl
	ld a,(lr_sid)
	inc a
	ld (lr_sid),a
	jp lo_entry
lo_bad_pop:
	pop de
	jr lo_bad
lo_more:
	ld a,(lr_sid)
	ld hl,lr_sbase
	cp (hl)
	jr z,lo_bad
	jp lo_block
lo_done:
	ld a,(lr_nsyms)
	ld l,a
	ld h,0
	add hl,hl
	ld bc,LIB_INDEX
	add hl,bc
	ld bc,(lr_sr)
	ld (hl),c
	inc hl
	ld (hl),b
	ld hl,(lr_sr)
	ld de,20
	add hl,de
	ld a,0
	adc a,0
	ld (lr_hdr),hl
	ld (lr_hdr+2),a
	ld hl,(lr_dir)
	ld a,(lr_dir+2)
	ld de,lr_hdr
	call lr_cmp24
	jr c,lo_bad
	ld hl,$FFFF
	ld (lr_dn),hl
	xor a
	ld (lr_wlen),a
	ld (lr_wp),a
	ld hl,$FFFF
	ld (lr_dcbase),hl
	call lsx_open
	ret nz
	ld hl,OPEN_MAGIC
	ld (lr_open),hl
	xor a
	ret
lo_bad:
	ld a,LIB_ERR_BAD
	or a
	ret

; in: A:HL = a, DE = address of a 24-bit value b. out: C if a < b. Destroys AF BC DE HL.
lr_cmp24:
	ld c,a
	push hl
	ex de,hl
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld b,(hl)
	pop hl
	or a
	sbc hl,de
	ld a,c
	sbc a,b
	ret

; out: Z if lib_open succeeded, else NZ and A = LIB_ERR_CLOSED. Destroys AF.
lr_need:
	ld a,(lr_open)
	cp OPEN_MAGIC & $FF
	jr nz,lr_closed
	ld a,(lr_open+1)
	cp OPEN_MAGIC >> 8
	jr nz,lr_closed
	xor a
	ret
lr_closed:
	ld a,LIB_ERR_CLOSED
	or a
	ret

lr_bad_index:
	ld a,LIB_ERR_INDEX
	or a
	ret

lib_assertion_count:
	call lr_need
	ret nz
	ld hl,(lr_count)
	xor a
	ret

lib_symbol_counts:
	call lr_need
	ret nz
	ld a,(lr_consts)
	ld b,a
	ld a,(lr_nsyms)
	sub b
	ld c,a
	xor a
	ret
