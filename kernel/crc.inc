; Library checksum: CRC-32 of the bytes from offset 8 to the end, against the header's value.

kernel_crc_begin:
	ld ix,crc_begin_body
	jp kernel_enter

kernel_crc_chunk:
	ld ix,crc_chunk_body
	jp kernel_enter

crc_refuse:
	ld hl,0
	ld (KERNEL_CRC),hl
	ld (KERNEL_CRC+2),hl
	ld d,h
	ld e,l
	ld a,ST_MALFORMED
	jp leave

#macro CRC_BYTE(k)
	ld a,(ix+k)
	xor e
	ld l,a
	ld h,CRC_TABLE>>8
	ld a,(hl)
	xor d
	ld e,a
	inc h
	ld a,(hl)
	xor c
	ld d,a
	inc h
	ld a,(hl)
	xor b
	ld c,a
	inc h
	ld b,(hl)
#endmacro

crc_begin_body:
	call is_ready
	jp nz,crc_refuse
	xor a
	ld (kv_crc_state),a
	ld hl,(kv_length)
	ld a,(kv_length+2)
	ld e,a
	ld a,h
	or l
	jr nz,cb_low
	dec e
cb_low:
	dec hl
	ld a,h
	rlca
	rlca
	and 3
	ld d,a
	ld a,e
	add a,a
	add a,a
	or d
	ld (kv_crc_last),a
	ld a,h
	and $3F
	or $80
	ld h,a
	inc hl
	ld (kv_crc_end),hl
	call crc_table_build
	ld hl,$FFFF
	ld (KERNEL_CRC),hl
	ld (KERNEL_CRC+2),hl
	xor a
	ld (kv_crc_chunk),a
	inc a
	ld (kv_crc_state),a
	ret

crc_chunk_body:
	call is_ready
	jp nz,crc_refuse
	ld a,(kv_crc_state)
	dec a
	jp nz,crc_refuse
	ld a,(kv_crc_chunk)
	ld c,a
	srl a
	srl a
	ld e,a
	ld a,c
	rrca
	rrca
	and $C0
	ld h,a
	ld l,0
	ld a,c
	or a
	jr nz,cc_map
	ld l,8
cc_map:
	call map_offset
	ld a,(kv_crc_last)
	ld b,a
	ld a,(kv_crc_chunk)
	ld de,$C000
	cp b
	jr nz,cc_run
	ld de,(kv_crc_end)
cc_run:
	call crc_update
	ld a,(kv_crc_chunk)
	ld hl,kv_crc_last
	cp (hl)
	jr z,cc_final
	inc a
	ld (kv_crc_chunk),a
	ld a,ST_CRC_MORE
	jp leave
cc_final:
	xor a
	ld (kv_crc_state),a
	ld hl,KERNEL_CRC
	ld b,4
cc_flip:
	ld a,(hl)
	cpl
	ld (hl),a
	inc hl
	djnz cc_flip
	ld hl,4
	ld e,0
	call map_offset
	ld de,KERNEL_CRC
	ld b,4
cc_compare:
	ld a,(de)
	cp (hl)
	jp nz,crc_refuse
	inc de
	inc hl
	djnz cc_compare
	ld hl,(KERNEL_CRC)
	ld de,(KERNEL_CRC+2)
	ret

; in: HL = first byte, DE = end (not before HL), KERNEL_CRC = running value.
; out: KERNEL_CRC updated. Destroys AF BC DE HL IX.
crc_update:
	push hl
	pop ix
	ex de,hl
	or a
	sbc hl,de
	ld a,l
	and 15
	ld (kv_crc_rem),a
	ld a,l
	and $F0
	ld l,a
	add hl,de
	ld (kv_crc_stop),hl
	or a
	sbc hl,de
	ld de,(KERNEL_CRC)
	ld bc,(KERNEL_CRC+2)
	jp z,cu_rest
cu_groups:
	CRC_BYTE(0)
	CRC_BYTE(1)
	CRC_BYTE(2)
	CRC_BYTE(3)
	CRC_BYTE(4)
	CRC_BYTE(5)
	CRC_BYTE(6)
	CRC_BYTE(7)
	CRC_BYTE(8)
	CRC_BYTE(9)
	CRC_BYTE(10)
	CRC_BYTE(11)
	CRC_BYTE(12)
	CRC_BYTE(13)
	CRC_BYTE(14)
	CRC_BYTE(15)
	push ix
	pop hl
	ld a,l
	add a,16
	ld l,a
	jr nc,cu_advance
	inc h
cu_advance:
	push hl
	pop ix
	ld a,(kv_crc_stop+1)
	cp h
	jp nz,cu_groups
	ld a,(kv_crc_stop)
	cp l
	jp nz,cu_groups
cu_rest:
	ld a,(kv_crc_rem)
	or a
	jr z,cu_store
cu_tail:
	CRC_BYTE(0)
	inc ix
	ld a,(kv_crc_rem)
	dec a
	ld (kv_crc_rem),a
	jr nz,cu_tail
cu_store:
	ld (KERNEL_CRC),de
	ld (KERNEL_CRC+2),bc
	ret

; out: the four byte planes of the CRC-32 table at CRC_TABLE. Destroys AF BC DE HL.
crc_table_build:
	ld hl,CRC_TABLE
ct_entry:
	ld e,l
	ld d,0
	ld bc,0
	ld a,8
	ld (kv_crc_bits),a
ct_bit:
	srl b
	rr c
	rr d
	rr e
	jr nc,ct_next
	ld a,e
	xor $20
	ld e,a
	ld a,d
	xor $83
	ld d,a
	ld a,c
	xor $B8
	ld c,a
	ld a,b
	xor $ED
	ld b,a
ct_next:
	ld a,(kv_crc_bits)
	dec a
	ld (kv_crc_bits),a
	jr nz,ct_bit
	ld (hl),e
	inc h
	ld (hl),d
	inc h
	ld (hl),c
	inc h
	ld (hl),b
	dec h
	dec h
	dec h
	inc l
	jr nz,ct_entry
	ret
