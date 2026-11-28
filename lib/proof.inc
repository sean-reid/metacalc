; A theorem's reference list and proof: where they are. See lib.z80 for the contract.

lp_page             .equ lr_dc
lp_addr             .equ lr_dc+1
lp_refs             .equ lr_dc+3
lp_nrefs            .equ lr_dc+6
lp_proof            .equ lr_dc+8
lp_plen             .equ lr_dc+11
lp_pos              .equ lr_tr
lp_np               .equ lr_cn
lp_nr               .equ lr_sr
lp_len              .equ lr_n
lp_start            .equ lr_hdr
lp_last             .equ lr_ent

; in: A:HL = offset. out: A = its 16 KB chunk, 8 bits. Destroys DE and F, keeps HL.
lp_chunk:
	ld e,a
	ld a,h
	rlca
	rlca
	and 3
	ld d,a
	ld a,e
	add a,a
	add a,a
	or d
	ret

; in: A:HL = a position, DE = a count of entries of two bytes. out: A:HL = position + 2 * DE.
; Destroys BC, DE.
lp_skip2:
	ld b,a
	ex de,hl
	xor a
	add hl,hl
	rla
	add hl,de
	adc a,b
	ret

lib_proof:
	call lib_record
	ret nz
	ld a,(lr_dkind)
	or a
	jr nz,lp_theorem
	ld a,LIB_ERR_NONE
	or a
	ret
lp_theorem:
	ld a,(lr_dclen)
	ld c,a
	ld b,0
	ld hl,(lr_dconc)
	ld a,(lr_dconc+2)
	call lr_adv
	call lw_seek
	ld a,2
	call lw_need
	ret nz
	call lw_ptr
	ld e,(hl)
	inc hl
	ld d,(hl)
	ld (lp_np),de
	ld bc,2
	call lw_skip
	call lw_pos
	ld de,(lp_np)
	call lp_skip2
	call lw_seek
	ld a,2
	call lw_need
	ret nz
	call lw_pos
	ld (lp_pos),hl
	ld (lp_pos+2),a
	call lw_ptr
	ld e,(hl)
	inc hl
	ld d,(hl)
	ld (lp_nr),de
	ld hl,(lp_pos)
	ld a,(lp_pos+2)
	ld bc,2
	call lr_adv
	call lp_skip2
	call lw_seek
	ld a,2
	call lw_need
	ret nz
	call lw_ptr
	ld e,(hl)
	inc hl
	ld d,(hl)
	ld (lp_len),de
	ld bc,2
	call lw_skip
	call lw_pos
	ld (lp_start),hl
	ld (lp_start+2),a
	ld bc,(lp_len)
	call lr_adv
	ld bc,1
	or a
	sbc hl,bc
	sbc a,0
	ld (lp_last),hl
	ld (lp_last+2),a
	ld de,(lr_length)
	or a
	sbc hl,de
	ld hl,lr_length+2
	sbc a,(hl)
	jp nc,lr_bad
	ld hl,(lp_last)
	ld a,(lp_last+2)
	call lp_chunk
	ld b,a
	ld hl,(lr_drec)
	ld a,(lr_drec+2)
	call lp_chunk
	cp b
	jp nz,lr_bad
	ld b,a
	ld a,(lr_page)
	sub b
	ld (lp_page),a
	ld a,(lr_drec+1)
	and $3F
	or $80
	ld h,a
	ld a,(lr_drec)
	ld l,a
	ld (lp_addr),hl
	ld hl,(lp_pos)
	ld a,(lp_pos+2)
	ld bc,2
	call lr_adv
	ld (lp_refs),hl
	ld (lp_refs+2),a
	ld hl,(lp_nr)
	ld (lp_nrefs),hl
	ld hl,(lp_start)
	ld (lp_proof),hl
	ld a,(lp_start+2)
	ld (lp_proof+2),a
	ld hl,(lp_len)
	ld (lp_plen),hl
	ld hl,$FFFF
	ld (lr_dcbase),hl
	xor a
	ret
