; The primitive. See lib.z80 for the contract.

lr_bad_arg:
	ld a,LIB_ERR_ARG
	ret
lr_bad_range:
	ld a,LIB_ERR_RANGE
	ret
lr_not_open:
	ld a,LIB_ERR_CLOSED
	ret

; The public entry: the reader is open and the destination stays clear of the state block.
lib_read:
	ld (lr_off),hl
	ld (lr_off+2),a
	ld a,(lr_open)
	cp OPEN_MAGIC & $FF
	jr nz,lr_not_open
	ld a,(lr_open+1)
	cp OPEN_MAGIC >> 8
	jr nz,lr_not_open
	call lr_overlap
	ret nz
	ld hl,(lr_off)
	ld a,(lr_off+2)

; The private entry, for reads into the state block and for the callers that checked their buffers.
lr_read:
	ld (lr_dest),de
	ld (lr_off),hl
	ld (lr_off+2),a
	dec bc
	ld a,b
	or a
	jr nz,lr_bad_arg
	ld a,d
	cp $80
	jr c,lr_bad_arg
	ex de,hl
	add hl,bc
	jr c,lr_bad_arg
	ld a,h
	cp $C0
	jr nc,lr_bad_arg
	ld hl,(lr_off)
	ld a,(lr_off+2)
	add hl,bc
	adc a,0
	jr c,lr_bad_range
	ld de,(lr_length)
	or a
	sbc hl,de
	ld hl,lr_length+2
	sbc a,(hl)
	jr nc,lr_bad_range

; From here to the DI nothing touches the shadow registers: an interrupt handler may use them.
lr_checked:
	ld hl,(lr_off)
	ld a,h
	rlca
	rlca
	and 3
	ld e,a
	ld a,(lr_off+2)
	add a,a
	add a,a
	or e
	ld e,a
	ld a,(lr_page)
	sub e
	ld (lr_page1),a
	ld a,h
	and $3F
	or $80
	ld h,a
	ld d,h
	ld e,l
	add hl,bc
	ld a,h
	cp $C0
	jr nc,lr_cross
	ex de,hl
	inc bc
	ld de,LIB_BOUNCE
	xor a
	ld (lr_n2),a
	jr lr_iff_save

; src + count reaches the next 16 KB chunk: BC = count - 1, DE = src.
lr_cross:
	ld hl,$C000
	or a
	sbc hl,de
	ld (lr_n1),hl
	ld (lr_src),de
	ld a,c
	sub l
	inc a
	ld (lr_n2),a
	ld bc,(lr_n1)
	ld hl,(lr_src)
	ld de,LIB_BOUNCE

lr_iff_save:
	ld a,i
	jp pe,lr_iff_on
	ld a,i
	jp pe,lr_iff_on
	xor a
	jr lr_iff_set
lr_iff_on:
	ld a,1
lr_iff_set:
	ld (lr_iff),a

lr_enter:
	di
	in a,(7)
	ex af,af'
	in a,(5)
	ld (lr_old5),a
	ld a,(lr_n2)
	or a
	jr z,lr_spare
	exx
	ld c,a
	ld b,0
	ld hl,(lr_n1)
	ld de,LIB_BOUNCE
	add hl,de
	ex de,hl
	exx
lr_spare:
	ld a,LIB_SPARE_PAGE
lr_window:
	out (5),a
	ld a,(lr_n2)
	or a
	ld a,(lr_page1)
	out (7),a
	ldir
	jr z,lr_hop2
	exx
	ld hl,$8000
	dec a
	out (7),a
	ldir
lr_hop2:
	ld hl,$10000-LIB_BOUNCE
	add hl,de
	ld b,h
	ld c,l
	ex af,af'
	out (7),a
	ld a,(lr_iff)
	or a
	ld a,(lr_old5)
	ld de,(lr_dest)
	ld hl,LIB_BOUNCE
	ldir
lr_window_end:
	out (5),a
	ld a,LIB_OK
	ret z
	ei
	ret

; in: DE = start, BC = length. out: Z, or NZ and A = LIB_ERR_ARG when [DE, DE + BC) touches the
; state block. Destroys HL.
lr_overlap:
	ld hl,LIB_STATE+LIB_STATE_SIZE
	or a
	sbc hl,de
	jr z,lro_clear
	jr c,lro_clear
	ld h,d
	ld l,e
	add hl,bc
	jr c,lro_touch
	push de
	ld de,LIB_STATE
	or a
	sbc hl,de
	pop de
	jr z,lro_clear
	jr c,lro_clear
lro_touch:
	ld a,LIB_ERR_ARG
	or a
	ret
lro_clear:
	xor a
	ret

; in: DE = start, BC = length, 1 to 256. out: Z, or NZ and A = LIB_ERR_ARG when the buffer is not
; wholly in bank B or touches the state block. Destroys HL.
lr_chkbuf:
	ld a,d
	cp $80
	jr c,lro_touch
	ld h,d
	ld l,e
	add hl,bc
	jr c,lro_touch
	ld a,h
	cp $C0
	jr c,lr_overlap
	jr nz,lro_touch
	ld a,l
	or a
	jr nz,lro_touch
	jr lr_overlap

; in: as lr_read. out: Z and A = LIB_OK, or NZ and A = LIB_ERR_BAD for any refusal. Destroys all but IX, IY.
lr_rd:
	call lr_read
	or a
	ret z
	ld a,LIB_ERR_BAD
	or a
	ret
