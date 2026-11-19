; Parse trees and the syntax table. See lib.z80 for the contract.

lib_syntax_count:
	call lr_need
	ret nz
	ld a,(ls_have)
	and LS_TREES
	jp z,lsx_nothing
	ld hl,(ls_tr_n)
	xor a
	ret

lib_syntax:
	ld (lr_cn),hl
	call lr_need
	ret nz
	ld a,(ls_have)
	and LS_TREES
	jp z,lsx_nothing
	ld hl,(lr_cn)
	ld de,(ls_tr_n)
	or a
	sbc hl,de
	jp nc,lr_bad_index
	ld hl,(lr_cn)
	ld d,h
	ld e,l
	add hl,hl
	add hl,de
	ld bc,2
	add hl,bc
	xor a
	ld de,ls_tr
	call lsx_add3
	ld de,lr_hdr
	ld bc,3
	call lr_rd
	ret nz
	ld hl,(lr_hdr)
	ld de,(lr_count)
	or a
	sbc hl,de
	jp nc,lr_bad
	ld a,(lr_hdr+2)
	ld b,a
	ld hl,(lr_hdr)
	xor a
	ret

lib_tree:
	ld (lr_bufp),de
	ld (lr_cn),hl
	ld a,b
	ld (lr_k),a
	call lr_need
	ret nz
	ld bc,255
	call lr_chkbuf
	ret nz
	ld a,(ls_have)
	and LS_TREES
	jp z,lsx_nothing
	ld hl,(lr_cn)
	ld de,(lr_count)
	or a
	sbc hl,de
	jp nc,lr_bad_index
	ld hl,(lr_cn)
	call lr_record
	ret nz
	call ltr_essentials
	ret nz
	ld a,(lr_k)
	cp $FF
	jr nz,ltr_known
	ld a,(lr_len)
	ld (lr_k),a
ltr_known:
	ld hl,lr_len
	cp (hl)
	jr z,ltr_in_range
	jp nc,lr_bad_index
ltr_in_range:
	ld hl,(ls_tr_n)
	ld d,h
	ld e,l
	add hl,hl
	add hl,de
	ld b,h
	ld c,l
	ld hl,(lr_cn)
	xor a
	add hl,hl
	rla
	add hl,hl
	rla
	call lr_adv
	ld bc,2
	call lr_adv
	ld de,ls_tr
	call lsx_add3
	jp c,lr_bad
	call lsx_entry4
	ret nz
	ld (lr_tr),hl
	ld (lr_tr+2),a
	ld bc,1
	ld de,ls_tr
	call lsx_within
	ret nz
	ld hl,(lr_tr)
	ld a,(lr_tr+2)
	call lw_seek
	ld a,(lr_len)
	ld (lr_id),a
	xor a
	ld (lr_want),a
ltr_all:
	call ltr_head
	ret nz
	ld a,(lr_want)
	ld hl,lr_k
	cp (hl)
	jr nz,ltr_next
	call lw_pos
	ld (lr_n),hl
	ld (lr_tc),a
	ld hl,(lr_hdr)
	ld (lr_sr),hl
	ld a,(lr_hdr+2)
	ld (lr_sid),a
ltr_next:
	ld bc,(lr_hdr)
	call lw_skip
	ld a,(lr_id)
	or a
	jr z,ltr_whole
	dec a
	ld (lr_id),a
	ld hl,lr_want
	inc (hl)
	jr ltr_all
ltr_whole:
	call lw_pos
	ld (ls_te),hl
	ld (ls_te+2),a
	ld hl,(lr_tr)
	ld a,(lr_tr+2)
	ld (ls_ts),hl
	ld (ls_ts+2),a
	call lsx_chunk
	ret nz
	ld hl,(ls_te)
	ld de,(ls_ts)
	or a
	sbc hl,de
	ld b,h
	ld c,l
	ld hl,(lr_tr)
	ld a,(lr_tr+2)
	ld de,ls_tr
	call lsx_within
	ret nz
	ld a,(lr_sid)
	or a
	jp nz,lsx_nothing
	ld hl,(lr_sr)
	ld a,h
	or l
	jp z,lr_bad
	ld hl,(lr_n)
	ld a,(lr_tc)
	ld (lr_tr),hl
	ld (lr_tr+2),a
	ld bc,(lr_sr)
	ld de,ls_tr
	call lsx_within
	ret nz
	ld bc,(lr_sr)
	ld a,b
	or a
	jr nz,ltr_cap
	ld a,c
	cp 255
	jr c,ltr_copy
ltr_cap:
	ld bc,255
ltr_copy:
	ld hl,(lr_tr)
	ld a,(lr_tr+2)
	ld de,(lr_bufp)
	call lr_rd
	ret nz
	ld bc,(lr_sr)
	ld hl,(lr_tr)
	ld a,(lr_tr+2)
	ld e,a
	ld d,0
	xor a
	ret

; The essential hypotheses of the described assertion.
; out: Z and lr_len = how many, or NZ and A = status. Destroys all but IX, IY.
ltr_essentials:
	ld hl,(lr_dhyps)
	ld a,(lr_dhyps+2)
	call lw_seek
	ld a,(lr_dnhyps)
	ld (lr_want),a
	xor a
	ld (lr_len),a
ltr_count:
	ld a,(lr_want)
	or a
	ret z
	call lhy_head
	ret nz
	push bc
	call lw_ptr
	ld a,(hl)
	ld hl,lr_len
	add a,(hl)
	ld (hl),a
	pop bc
	call lw_skip
	ld hl,lr_want
	dec (hl)
	jr ltr_count

; The header of the tree at the window's cursor, which moves to the tree's first byte.
; out: Z, lr_hdr = the length (2), lr_hdr+2 = 1 for no tree else 0; or NZ and A = status.
; Destroys all but IX, IY.
ltr_head:
	ld a,3
	call lw_need
	ret nz
	call lw_ptr
	ld a,(hl)
	ld bc,1
	cp $FE
	jr z,ltr_empty
	cp $FF
	jr z,ltr_long
	ld (lr_hdr),a
	xor a
	ld (lr_hdr+1),a
	ld (lr_hdr+2),a
	jr ltr_header
ltr_long:
	inc hl
	ld a,(hl)
	ld (lr_hdr),a
	inc hl
	ld a,(hl)
	ld (lr_hdr+1),a
	xor a
	ld (lr_hdr+2),a
	ld bc,3
	jr ltr_header
ltr_empty:
	xor a
	ld (lr_hdr),a
	ld (lr_hdr+1),a
	inc a
	ld (lr_hdr+2),a
ltr_header:
	call lw_skip
	xor a
	ret

lib_node:
	call lr_need
	ret nz
	ld a,c
	or a
	jp z,lr_bad_range
	ld a,(hl)
	cp $7F
	jr z,lnd_escape
	cp $FF
	jr z,lnd_escape
	inc hl
	dec c
	cp $80
	jr nc,lnd_inline
	ld e,a
	ld d,0
	jr lnd_axiom
lnd_escape:
	ld a,c
	cp 2
	jp c,lr_bad_range
	ld b,(hl)
	inc hl
	ld a,(hl)
	inc hl
	dec c
	dec c
	add a,127
	ld e,a
	ld d,0
	jr nc,lnd_wide
	inc d
lnd_wide:
	ld a,b
	cp $7F
	jr z,lnd_axiom
	ld a,d
	or a
	jp nz,lr_bad
	ld a,e
	jr lnd_symbol
lnd_axiom:
	push hl
	ld hl,(ls_tr_n)
	or a
	sbc hl,de
	pop hl
	jp z,lr_bad
	jp c,lr_bad
	ld b,0
	xor a
	ret
lnd_inline:
	sub $80
lnd_symbol:
	ld e,a
	ld a,(lr_consts)
	add a,e
	jp c,lr_bad
	ld e,a
	ld d,0
	ld a,(lr_nsyms)
	cp e
	jp z,lr_bad
	jp c,lr_bad
	ld b,1
	xor a
	ret

lib_sections:
	call lr_need
	ret nz
	ld a,(ls_have)
	and LS_TITLES|LS_GLYPHS|LS_TREES
	ld b,a
	xor a
	ret
