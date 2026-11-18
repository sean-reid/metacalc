; Assertions: directory entries, record heads, labels, statements.

; in: A:HL += BC. Destroys F.
lr_adv:
	add hl,bc
	adc a,0
	ret

lr_bad:
	ld a,LIB_ERR_BAD
	or a
	ret

; in: HL = assertion number. Directory entries are cached eight at a time in lr_dc.
; out: Z and A:HL = the record's offset, or NZ and A = status. Destroys all but IX, IY.
lr_recoff:
	ld bc,(lr_count)
	push hl
	or a
	sbc hl,bc
	pop hl
	jp nc,lr_bad_index
	ld (lr_q),hl
	ld a,l
	and $F8
	ld l,a
	ld de,(lr_dcbase)
	or a
	sbc hl,de
	jr z,lre_hit
	ld hl,$FFFF
	ld (lr_dcbase),hl
	ld hl,(lr_q)
	ld a,l
	and $F8
	ld l,a
	ld de,(lr_count)
	ex de,hl
	or a
	sbc hl,de
	ld a,h
	or a
	ld a,8
	jr nz,lre_take
	ld a,l
	cp 8
	jr c,lre_take
	ld a,8
lre_take:
	add a,a
	add a,a
	ld c,a
	ld b,0
	ex de,hl
	xor a
	add hl,hl
	rla
	add hl,hl
	rla
	ld de,(lr_dir)
	add hl,de
	ld e,a
	ld a,(lr_dir+2)
	adc a,e
	ld de,lr_dc
	call lr_rd
	ret nz
	ld hl,(lr_q)
	ld a,l
	and $F8
	ld l,a
	ld (lr_dcbase),hl
lre_hit:
	ld a,(lr_q)
	and 7
	add a,a
	add a,a
	ld e,a
	ld d,0
	ld hl,lr_dc
	add hl,de
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld b,(hl)
	inc hl
	ld a,(hl)
	or a
	jp nz,lr_bad
	ex de,hl
	ld a,b
	cp a
	ret

lib_label:
	ld (lr_bufp),de
	ld (lr_cn),hl
	call lr_need
	ret nz
	ld bc,255
	call lr_chkbuf
	ret nz
	ld hl,(lr_cn)
	call lr_recoff
	ret nz
	ld (lr_tr),hl
	ld (lr_tr+2),a
	ld de,lr_hdr
	ld bc,2
	call lr_rd
	ret nz
	ld a,(lr_hdr+1)
	or a
	jr z,llb_done
	ld c,a
	ld b,0
	ld hl,(lr_tr)
	ld a,(lr_tr+2)
	ld de,2
	add hl,de
	adc a,0
	ld de,(lr_bufp)
	call lr_rd
	ret nz
llb_done:
	ld a,(lr_hdr+1)
	ld b,a
	xor a
	ret

lib_record:
	call lr_need
	ret nz
lr_record:
	ld bc,(lr_count)
	push hl
	or a
	sbc hl,bc
	pop hl
	jr c,lrc_in_range
	ld hl,$FFFF
	ld (lr_dn),hl
	jp lr_bad_index
lrc_in_range:
	ld de,(lr_dn)
	push hl
	or a
	sbc hl,de
	pop hl
	jr nz,lrc_fill
	xor a
	ret
lrc_fill:
	ld (lr_n),hl
	ld hl,$FFFF
	ld (lr_dn),hl
	ld hl,(lr_n)
	call lr_recoff
	ret nz
	ld (lr_drec),hl
	ld (lr_drec+2),a
	call lw_seek
	ld a,2
	call lw_need
	ret nz
	call lw_ptr
	ld a,(hl)
	cp 2
	jp nc,lr_bad
	ld (lr_dkind),a
	inc hl
	ld a,(hl)
	ld (lr_dllen),a
	ld c,a
	ld b,0
	inc bc
	inc bc
	call lw_skip
	ld a,1
	call lw_need
	ret nz
	call lw_ptr
	ld a,(hl)
	ld (lr_dnhyps),a
	ld (lr_want),a
	ld bc,1
	call lw_skip
	call lw_pos
	ld (lr_dhyps),hl
	ld (lr_dhyps+2),a
lrc_walk:
	ld a,(lr_want)
	or a
	jr z,lrc_pairs
	call lhy_head
	ret nz
	call lw_skip
	ld hl,lr_want
	dec (hl)
	jr lrc_walk
lrc_pairs:
	ld a,2
	call lw_need
	ret nz
	call lw_ptr
	ld e,(hl)
	inc hl
	ld d,(hl)
	ld (lr_dnpairs),de
	ld bc,2
	call lw_skip
	call lw_pos
	ld (lr_dpairs),hl
	ld (lr_dpairs+2),a
	ld hl,(lr_dnpairs)
	add hl,hl
	ld a,0
	rla
	ld bc,(lr_dpairs)
	add hl,bc
	ld c,a
	ld a,(lr_dpairs+2)
	adc a,c
	call lw_seek
	ld a,1
	call lw_need
	ret nz
	call lw_ptr
	ld a,(hl)
	ld (lr_dclen),a
	ld bc,1
	call lw_skip
	call lw_pos
	ld (lr_dconc),hl
	ld (lr_dconc+2),a
	xor a
	ld (lr_hk),a
	ld hl,(lr_dhyps)
	ld (lr_hoff),hl
	ld a,(lr_dhyps+2)
	ld (lr_hoff+2),a
	ld hl,(lr_n)
	ld (lr_dn),hl
	xor a
	ret

lib_conclusion:
	ld (lr_bufp),de
	ld (lr_cn),hl
	call lr_need
	ret nz
	ld bc,255
	call lr_chkbuf
	ret nz
	ld hl,(lr_cn)
	call lib_record
	ret nz
	ld a,(lr_dclen)
	or a
	jr z,lcn_done
	ld c,a
	ld b,0
	ld de,(lr_bufp)
	ld hl,(lr_dconc)
	ld a,(lr_dconc+2)
	call lr_rd
	ret nz
lcn_done:
	ld a,(lr_dclen)
	ld b,a
	xor a
	ret

lib_hypothesis:
	ld (lr_bufp),de
	ld (lr_cn),hl
	ld a,b
	ld (lr_k),a
	call lr_need
	ret nz
	ld bc,255
	call lr_chkbuf
	ret nz
	ld hl,(lr_cn)
	call lib_record
	ret nz
	ld a,(lr_k)
	ld hl,lr_dnhyps
	cp (hl)
	jp nc,lr_bad_index
	ld hl,lr_hk
	cp (hl)
	jr nc,lhy_skip
	xor a
	ld (lr_hk),a
	ld hl,(lr_dhyps)
	ld (lr_hoff),hl
	ld a,(lr_dhyps+2)
	ld (lr_hoff+2),a
lhy_skip:
	ld a,(lr_hk)
	ld hl,lr_k
	cp (hl)
	jr z,lhy_here
	call lhy_step
	ret nz
	jr lhy_skip
lhy_here:
	ld hl,(lr_hoff)
	ld a,(lr_hoff+2)
	call lw_seek
	call lhy_head
	ret nz
	call lw_ptr
	ld a,(hl)
	or a
	jr nz,lhy_essential
	inc hl
	ld a,(hl)
	ld (lr_tc),a
	xor a
	ld (lr_len),a
	jr lhy_done
lhy_essential:
	ld a,$FF
	ld (lr_tc),a
	inc hl
	ld a,(hl)
	ld (lr_len),a
	or a
	jr z,lhy_done
	ld c,a
	ld b,0
	ld hl,(lr_hoff)
	ld a,(lr_hoff+2)
	ld de,2
	add hl,de
	adc a,0
	ld de,(lr_bufp)
	call lr_rd
	ret nz
lhy_done:
	call lhy_step
	ret nz
	ld a,(lr_len)
	ld b,a
	ld a,(lr_tc)
	ld c,a
	xor a
	ret

; out: Z and BC = the size of the hypothesis at the cursor.
lhy_head:
	ld a,2
	call lw_need
	ret nz
	call lw_ptr
	ld a,(hl)
	or a
	ld bc,2
	ret z
	cp 1
	jp nz,lr_bad
	inc hl
	ld c,(hl)
	ld b,0
	inc bc
	inc bc
	xor a
	ret

; Moves lr_hk and lr_hoff past one hypothesis.
lhy_step:
	ld hl,(lr_hoff)
	ld a,(lr_hoff+2)
	call lw_seek
	call lhy_head
	ret nz
	ld hl,(lr_hoff)
	ld a,(lr_hoff+2)
	call lr_adv
	ld (lr_hoff),hl
	ld (lr_hoff+2),a
	ld hl,lr_hk
	inc (hl)
	xor a
	ret
