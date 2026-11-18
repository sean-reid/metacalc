; Section titles. See lib.z80 for the contract.

; in: HL = title number, below the count. out: Z, lr_tr = the item's offset, lr_hdr = its
; assertion number (2), depth, text length; the item lies inside the section. Else NZ and
; A = status. Destroys all but IX, IY.
ltl_head:
	xor a
	add hl,hl
	rla
	add hl,hl
	rla
	ld de,ls_ti
	call lsx_add3
	jp c,lr_bad
	ld bc,2
	call lr_adv
	call lsx_entry4
	ret nz
	ld (lr_tr),hl
	ld (lr_tr+2),a
	ld de,lr_hdr
	ld bc,4
	call lr_rd
	ret nz
	ld a,(lr_hdr+3)
	ld c,a
	ld b,0
	inc bc
	inc bc
	inc bc
	inc bc
	ld hl,(lr_tr)
	ld a,(lr_tr+2)
	ld de,ls_ti
	jp lsx_within

lib_title_count:
	call lr_need
	ret nz
	ld a,(ls_have)
	and LS_TITLES
	jp z,lsx_nothing
	ld hl,(ls_ti_n)
	xor a
	ret

lib_title:
	ld (lr_bufp),de
	ld (lr_cn),hl
	call lr_need
	ret nz
	ld bc,255
	call lr_chkbuf
	ret nz
	ld a,(ls_have)
	and LS_TITLES
	jp z,lsx_nothing
	ld hl,(lr_cn)
	ld de,(ls_ti_n)
	or a
	sbc hl,de
	jp nc,lr_bad_index
	ld hl,(lr_cn)
	call ltl_head
	ret nz
	ld a,(lr_hdr+2)
	dec a
	cp 4
	jp nc,lr_bad
	ld hl,(lr_hdr)
	ld de,(lr_count)
	or a
	sbc hl,de
	jp nc,lr_bad
	ld a,(lr_hdr+3)
	or a
	jr z,ltl_done
	ld c,a
	ld b,0
	push bc
	ld bc,4
	ld hl,(lr_tr)
	ld a,(lr_tr+2)
	call lr_adv
	pop bc
	ld de,(lr_bufp)
	call lr_rd
	ret nz
ltl_done:
	ld hl,(lr_hdr)
	ld a,(lr_hdr+3)
	ld b,a
	ld a,(lr_hdr+2)
	ld c,a
	xor a
	ret

lib_title_for:
	ld (ls_k),hl
	call lr_need
	ret nz
	ld a,(ls_have)
	and LS_TITLES
	jp z,lsx_nothing
	ld hl,(ls_k)
	ld de,(lr_count)
	or a
	sbc hl,de
	jp nc,lr_bad_index
	ld hl,0
	ld (ls_lo),hl
	ld hl,(ls_ti_n)
	ld (ls_hi),hl
ltf_loop:
	ld hl,(ls_lo)
	ld de,(ls_hi)
	or a
	sbc hl,de
	jr nc,ltf_end
	ld hl,(ls_lo)
	add hl,de
	rr h
	rr l
	ld (ls_mid),hl
	call ltl_head
	ret nz
	ld hl,(ls_k)
	ld de,(lr_hdr)
	or a
	sbc hl,de
	jr c,ltf_above
	ld hl,(ls_mid)
	inc hl
	ld (ls_lo),hl
	jr ltf_loop
ltf_above:
	ld hl,(ls_mid)
	ld (ls_hi),hl
	jr ltf_loop
ltf_end:
	ld hl,(ls_lo)
	ld a,h
	or l
	jp z,lsx_nothing
	dec hl
	xor a
	ret
