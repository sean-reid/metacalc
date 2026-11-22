; The open routine of store.inc, which includes this file.
; In: DE = a size word's address, HL = three words: count, CRC low, CRC high. Out: A = 0 when
; the variable there is this library's table, else the reason it is not.
so_check:
	ex de, hl
	ld c, (hl)
	inc hl
	ld b, (hl)
	inc hl
	push hl
	pop ix
	ld a, b
	or a
	jr nz, ck_long
	ld a, c
	cp STORE_HEADER
	jr c, ck_size
ck_long:
	ld a, (ix + 0)
	cp STORE_MAGIC
	jr nz, ck_magic
	ld a, (ix + 1)
	cp STORE_VERSION
	jr nz, ck_magic
	ld hl, 2
	add hl, de
	ld a, (hl)
	cp (ix + 2)
	jr nz, ck_other
	inc hl
	ld a, (hl)
	cp (ix + 3)
	jr nz, ck_other
	inc hl
	ld a, (hl)
	cp (ix + 4)
	jr nz, ck_other
	inc hl
	ld a, (hl)
	cp (ix + 5)
	jr nz, ck_other
	ld a, (de)
	cp (ix + 6)
	jr nz, ck_other
	inc de
	ld a, (de)
	cp (ix + 7)
	jr nz, ck_other
	push bc
	ld c, (ix + 6)
	ld b, (ix + 7)
	call store_size_for
	pop hl
	or a
	sbc hl, bc
	jr nz, ck_size
	xor a
	ret
ck_magic:
	ld a, ST_BAD_MAGIC
	ret
ck_other:
	ld a, ST_OTHER_LIBRARY
	ret
ck_size:
	ld a, ST_BAD_SIZE
	ret

; The frame of store_open, from the top of the stack: a flags byte and a reason byte, which hold
; the archived size while an archived table is being moved, the count, the CRC low word, the
; CRC high word. In: nothing. Out: HL = the address of the frame
; when called with the frame on top of the stack.
so_frame:
	ld hl, 2
	add hl, sp
	ret

; In: the frame on top of the stack under the return address. Out: BC = the table size.
so_size:
	call so_frame
	inc hl
	inc hl
	inc hl
	inc hl
	ld c, (hl)
	inc hl
	ld b, (hl)
	jp store_size_for

; Moves an archived MCSTAT to RAM when the larger of the table the frame's count implies and the
; archived size fits. In: the frame under the return address, its first word holding the
; archived size.
so_unarchive:
	call so_frame
	inc hl
	inc hl
	ld e, (hl)
	inc hl
	ld d, (hl)
	inc hl
	ld c, (hl)
	inc hl
	ld b, (hl)
	call store_size_for
	ld h, b
	ld l, c
	or a
	sbc hl, de
	jr c, su_have
	ld d, b
	ld e, c
su_have:
	ld hl, STORE_OVERHEAD
	add hl, de
	jr c, su_no
	bcall(_EnoughMem)
	jr c, su_no
	call store_name
	bcall(_Arc_Unarc)
	xor a
	ret
su_no:
	ld a, ST_ARCHIVED
	ret

store_open:
	push de
	push hl
	push bc
	ld hl, 0
	push hl
	call store_name
	bcall(_ChkFindSym)
	ld a, ST_ABSENT
	jp c, so_create
	ld a, b
	or a
	jp z, so_found
	ld h, d
	ld l, e
	ld de, STORE_ARC_SIZE
	add hl, de
	bcall(_LoadDEIndPaged)
	ld hl, 0
	add hl, sp
	ld (hl), e
	inc hl
	ld (hl), d
	call so_unarchive
	or a
	jp nz, so_fail
	ld hl, 0
	add hl, sp
	ld (hl), OPEN_WAS_ARCHIVED
	inc hl
	ld (hl), 0
	call store_name
	bcall(_ChkFindSym)
	ld a, ST_ABSENT
	jp c, so_create
	ld a, b
	or a
	ld a, ST_ARCHIVED
	jp nz, so_fail
so_found:
	push hl
	push de
	ld hl, 6
	add hl, sp
	call so_check
	pop de
	pop hl
	or a
	jp z, so_kept
	push af
	ld b, 0
	bcall(_DelVarArc)
	pop af
so_create:
	ld hl, 1
	add hl, sp
	ld (hl), a
	call so_size
	ld hl, STORE_OVERHEAD
	add hl, bc
	bcall(_EnoughMem)
	ld a, ST_NO_RAM
	jp c, so_fail
	call store_name
	call so_size
	ld h, b
	ld l, c
	bcall(_CreateAppVar)
	inc de
	inc de
	ld a, STORE_MAGIC
	ld (de), a
	inc de
	ld a, STORE_VERSION
	ld (de), a
	inc de
	call so_frame
	inc hl
	inc hl
	push hl
	inc hl
	inc hl
	ld bc, 4
	ldir
	pop hl
	ldi
	ldi
	push de
	call so_frame_below
	pop de
	ld c, (hl)
	inc hl
	ld b, (hl)
	call store_size_for
	ld hl, -STORE_OFF_NEXT
	add hl, bc
	ld b, h
	ld c, l
	ex de, hl
	ld (hl), 0
	ld d, h
	ld e, l
	inc de
	dec bc
	ldir
so_kept:
	pop hl
	ld a, l
	or h
	ld b, a
	pop hl
	pop hl
	pop hl
	xor a
	ret
so_fail:
	ld b, 0
	pop hl
	pop hl
	pop hl
	pop hl
	ret

; In: one word pushed above the frame. Out: HL = the address of the count in the frame.
so_frame_below:
	ld hl, 6
	add hl, sp
	ret

store_has:
	push de
	push hl
	push bc
	call store_name
	bcall(_ChkFindSym)
	ld a, ST_ABSENT
	jr c, sh_end
	ld a, b
	or a
	ld a, ST_ARCHIVED
	jr nz, sh_end
	ld hl, 0
	add hl, sp
	call so_check
sh_end:
	pop bc
	pop hl
	pop de
	ret
