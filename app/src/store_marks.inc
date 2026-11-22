; The mark lookups of store.inc, which includes this file: whether a list of assertions all hold
; the value 1, and where the next unchecked assertion is.
SAO_HDR  = OP2
SAO_CNT  = OP2 + 2
SAO_LEFT = OP2 + 4

SNZ_HDR  = OP2
SNZ_CNT  = OP2 + 2
SNZ_END  = OP2 + 4
SNZ_LEFT = OP2 + 6
SNZ_K    = OP2 + 8
SNZ_P    = OP2 + 10

store_all_ones:
	push hl
	push bc
	call store_data
	pop de
	pop ix
	or a
	jr nz, sao_fail
	ld (SAO_HDR), hl
	ld (SAO_CNT), bc
	ld (SAO_LEFT), de
sao_next:
	ld hl, (SAO_LEFT)
	ld a, h
	or l
	jr z, sao_all
	dec hl
	ld (SAO_LEFT), hl
	ld e, (ix + 0)
	ld d, (ix + 1)
	inc ix
	inc ix
	bit 7, d
	jr nz, sao_next
	ld hl, (SAO_HDR)
	ld bc, (SAO_CNT)
	call store_locate
	or a
	jr nz, sao_fail
	call store_unpack
	cp 1
	jr z, sao_next
	ld b, 0
	xor a
	ret
sao_all:
	ld b, 1
	xor a
	ret
sao_fail:
	ld b, 0
	ret

store_next_zero:
	push hl
	push de
	call store_data
	pop de
	pop ix
	or a
	ret nz
	ld (SNZ_HDR), hl
	ld (SNZ_CNT), bc
	ld (SNZ_END), de
	ld h, b
	ld l, c
	or a
	sbc hl, de
	jr c, snz_range
	push ix
	pop hl
	or a
	sbc hl, de
	jr z, snz_found
	jr nc, snz_range
	push ix
	pop de
snz_next:
	ld hl, (SNZ_END)
	or a
	sbc hl, de
	jr z, snz_found
	ld (SNZ_LEFT), hl
	call snz_at
	ld a, c
	or a
	jr nz, snz_one
	ld a, (SNZ_LEFT + 1)
	or a
	jr nz, snz_whole
	ld a, (SNZ_LEFT)
	cp 4
	jr c, snz_one
snz_whole:
	ld bc, (SNZ_LEFT)
	srl b
	rr c
	srl b
	rr c
	ld (SNZ_K), bc
	ld (SNZ_P), de
snz_byte:
	ld a, (hl)
	ld d, a
	rrca
	or d
	and 55h
	cp 55h
	jr nz, snz_hit
	inc hl
	dec bc
	ld a, b
	or c
	jr nz, snz_byte
	call snz_skip
	jr snz_next
snz_hit:
	call snz_skip
snz_one:
	call snz_at
	call store_unpack
	or a
	jr z, snz_found
	inc de
	jr snz_next
snz_found:
	ex de, hl
	xor a
	ret
snz_range:
	ld a, ST_RANGE
	ret

; In: DE = an assertion number below the count. Out: HL = its table byte, C = its shift. Keeps DE.
snz_at:
	push de
	ld hl, (SNZ_HDR)
	ld bc, (SNZ_CNT)
	call store_locate
	pop de
	ret

; In: BC = the whole bytes not yet passed of the SNZ_K the scan started with. Out: DE = the
; number of the first assertion of the first of them.
snz_skip:
	ld hl, (SNZ_K)
	or a
	sbc hl, bc
	add hl, hl
	add hl, hl
	ld de, (SNZ_P)
	add hl, de
	ex de, hl
	ret
