; The checks of statecheck.inc: the checksums, the fill, the comparison with the patterns and the
; region table. It uses statecheck.inc's scratch and is included by it.

; Compares what a pass finds with the patterns and the checksums with their earlier values.
update:
	ld a, (iy + scriptFlags)
	ld (S_FLAG), a
	call check_all
	ld hl, statVars
	ld bc, STAT_VARS_SIZE
	call fletcher
	ld hl, S_STAT
	ld de, S_SUM
	call same4
	ld a, 0
	jr z, stat_kept
	inc a
stat_kept:
	ld (S_STAT_NOW), a
	call ssc_sum
	ld hl, S_SSC
	ld de, S_SUM
	call same4
	ld a, 0
	jr z, ssc_kept
	inc a
ssc_kept:
	ld (S_SSC_NOW), a
	jr copy_sum_to_ssc

ssc_sum:
	ld hl, SSC_TAIL
	ld bc, REPORT_TAIL
	jr fletcher

copy_sum_to_stat:
	ld de, S_STAT
	jr copy_sum
copy_sum_to_ssc:
	ld de, S_SSC
copy_sum:
	ld hl, S_SUM
	ld bc, 4
	ldir
	ret

; Contract: HL and DE point at 4 bytes; Z when they are equal.
same4:
	ld b, 4
same4_next:
	ld a, (de)
	cp (hl)
	ret nz
	inc hl
	inc de
	djnz same4_next
	ret

; Contract: the byte sum of BC bytes at HL and the sum of its running values, 16 bits each, to S_SUM.
fletcher:
	ld de, 0
	ld ix, 0
fletcher_next:
	ld a, (hl)
	add a, e
	ld e, a
	jr nc, fletcher_carried
	inc d
fletcher_carried:
	add ix, de
	inc hl
	dec bc
	ld a, b
	or c
	jr nz, fletcher_next
	ld (S_SUM), de
	ld (S_SUM + 2), ix
	ret

; Contract: HL is a region table entry, address and length words then the seed byte. Returns DE the
; address, BC the length, A the seed and HL the next entry.
region_get:
	ld e, (hl)
	inc hl
	ld d, (hl)
	inc hl
	ld c, (hl)
	inc hl
	ld b, (hl)
	inc hl
	ld a, (hl)
	inc hl
	ret

fill_all:
	ld hl, regions
	ld b, REGION_COUNT
fill_next:
	push bc
	call region_get
	push hl
	ex de, hl
fill_byte:
	ld (hl), a
	add a, STEP
	inc hl
	dec bc
	ld d, a
	ld a, b
	or c
	ld a, d
	jr nz, fill_byte
	pop hl
	pop bc
	djnz fill_next
	ret

; Writes S_RES, 3 bytes per compared region: the first offset that differs, and the count.
check_all:
	ld ix, S_RES
	ld hl, regions
	ld b, COMPARED
check_next:
	push bc
	call region_get
	push hl
	ex de, hl
	ld d, a
	ld (ix + 0), 0
	ld (ix + 1), 0
	ld (ix + 2), 0
	ld e, 0
check_byte:
	ld a, (hl)
	cp d
	jr z, check_same
	ld a, (ix + 1)
	or (ix + 2)
	jr nz, check_counted
	ld (ix + 0), e
check_counted:
	inc (ix + 1)
	jr nz, check_same
	inc (ix + 2)
check_same:
	ld a, d
	add a, STEP
	ld d, a
	inc e
	inc hl
	dec bc
	ld a, b
	or c
	jr nz, check_byte
	ld de, 3
	add ix, de
	pop hl
	pop bc
	djnz check_next
	ld a, 0
	ld hl, S_RES
	ld b, COMPARED * 3
fill_clean:
	or (hl)
	inc hl
	djnz fill_clean
	ld (S_FILL), a
	ret

; Contract: HL is a name of 10 bytes, the type then the name padded with zeros. Returns A: 0 not
; found, 1 in RAM, 2 archived.
find_var:
	ld de, OP1
	ld bc, 10
	ldir
	bcall(_ChkFindSym)
	ld a, 0
	ret c
	ld a, b
	or a
	ld a, 1
	ret z
	inc a
	ret

; Address, length and seed of each region. The first COMPARED are the ones the key screen reports.
regions:
	.dw SCR_VARS, SCR_VARS_SIZE
	.db 77h
	.dw SCR_STACK, SCR_STACK_SIZE
	.db 22h
	.dw appData, APP_DATA_SIZE
	.db 44h
	.dw textShadow, TEXT_SHADOW_SIZE
	.db 11h
	.dw SSC_TAIL, REPORT_TAIL
	.db 88h
