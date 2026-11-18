; The status table: which assertions of the library have been checked, in one AppVar that
; lives in RAM and survives app launches and power cycles. The including file has equates.inc
; and runs in the app page.
;
; The AppVar is MCSTAT. Its bytes after the OS's size word are
;     +0  magic 4Dh, then version 01h
;     +2  CRC-32 of the library, low byte first
;     +6  assertion count, 2 bytes
;     +8  number of the next assertion to check, 2 bytes
;    +10  2 bits per assertion, four to a byte, assertion 0 in the low bits:
;         0 unchecked, 1 checked, 2 failed, 3 checked but resting on something not checked
; so the size is 10 + ceil(count / 4); 842 bytes for 3,326 assertions.
;
; The module keeps no state and no pointer. The table moves whenever the OS creates or deletes
; a variable, so every routine finds MCSTAT again through store_data, the one routine that
; returns the data pointer, and uses that pointer only until its next OS call. Callers hold no
; pointer into the table either. Each routine checks the size, magic and version it finds.
;
; Status, in A, for every routine:
ST_OK            = 0
ST_NO_RAM        = 1    ; open: not enough RAM for the table, nothing created
ST_ARCHIVED      = 2    ; the variable is archived and could not be unarchived
ST_BAD_MAGIC     = 3    ; wrong magic or version
ST_OTHER_LIBRARY = 4    ; another library's CRC or another assertion count
ST_BAD_SIZE      = 5    ; the size is not what the header implies
ST_RANGE         = 6    ; an assertion number, a value or a next number out of range
ST_ABSENT        = 7    ; no variable: open was not called, or the user deleted it
;
; All routines destroy every register but IY, which must be the OS flags pointer, and the
; stack. They destroy OP1 and OP2, and store_count also OP3.
;
; store_open   in:  DE:HL = the library's CRC-32 (DE the high 16 bits), BC = assertion count
;              out: A = 0 or ST_NO_RAM or ST_ARCHIVED; with 0, B = how it went:
;                   the old table's fate in bits 0-6: 0 kept as it was, ST_ABSENT created because
;                   there was none, ST_BAD_MAGIC, ST_OTHER_LIBRARY or ST_BAD_SIZE deleted and
;                   created zeroed for that reason; bit 7 (OPEN_WAS_ARCHIVED) set when the user
;                   had archived it and it was moved back to RAM.
;              The table of a new variable is all unchecked and its next number is 0. An
;              archived variable is unarchived only when the RAM for a table of this count is
;              free; otherwise nothing changes and ST_ARCHIVED comes back.
; store_get    in:  HL = assertion number    out: A = status, B = value 0 to 3
; store_set    in:  HL = assertion number, B = value 0 to 3    out: A = status
; store_get_next  out: A = status, HL = number of the next assertion to check
; store_set_next  in:  HL = number, at most the count    out: A = status
; store_count  in:  HL = 8 bytes outside the table and OP1 to OP3
;              out: A = status; the buffer holds four little-endian words, the number of
;                   assertions with value 0, 1, 2 and 3
; store_reset  out: A = status; every assertion unchecked and the next number 0
OPEN_WAS_ARCHIVED = 80h

STORE_MAGIC    = 4Dh
STORE_VERSION  = 01h
STORE_HEADER   = 10
STORE_OFF_CRC  = 2
STORE_OFF_CNT  = 6
STORE_OFF_NEXT = 8
STORE_NAME_LEN = 6
; An archived variable's size word follows 10 bytes and the name in its archive entry, which
; ChkFindSym's data pointer addresses.
STORE_ARC_SIZE = 10 + STORE_NAME_LEN
; The size word, the 13-byte symbol table entry of a six-letter name, and a byte of slack.
STORE_OVERHEAD = 16

store_name:
	ld hl, store_name_text
	ld de, OP1
	ld bc, 9
	ldir
	ret

store_name_text:
	.db appVarObj, "MCSTAT", 0, 0

; Out: HL = the table's header and BC = the assertion count, valid until the next OS call, or
; A = ST_ABSENT, ST_ARCHIVED, ST_BAD_MAGIC or ST_BAD_SIZE and the rest destroyed.
store_data:
	call store_name
	bcall(_ChkFindSym)
	jr c, sd_absent
	ld a, b
	or a
	jr nz, sd_archived
	ex de, hl
	ld e, (hl)
	inc hl
	ld d, (hl)
	inc hl
	ld a, d
	or a
	jr nz, sd_long
	ld a, e
	cp STORE_HEADER
	jr c, sd_bad_size
sd_long:
	ld a, (hl)
	cp STORE_MAGIC
	jr nz, sd_bad_magic
	inc hl
	ld a, (hl)
	dec hl
	cp STORE_VERSION
	jr nz, sd_bad_magic
	call store_count_of
	call store_size_for
	ld a, e
	cp c
	jr nz, sd_bad_size
	ld a, d
	cp b
	jr nz, sd_bad_size
	call store_count_of
	xor a
	ret
sd_absent:
	ld a, ST_ABSENT
	ret
sd_archived:
	ld a, ST_ARCHIVED
	ret
sd_bad_magic:
	ld a, ST_BAD_MAGIC
	ret
sd_bad_size:
	ld a, ST_BAD_SIZE
	ret

; In: HL = header. Out: BC = assertion count. Keeps HL and DE.
store_count_of:
	push hl
	ld bc, STORE_OFF_CNT
	add hl, bc
	ld c, (hl)
	inc hl
	ld b, (hl)
	pop hl
	ret

; In: BC = assertion count. Out: BC = 10 + ceil(count / 4). Keeps DE and HL.
store_size_for:
	ld a, c
	and 3
	push af
	srl b
	rr c
	srl b
	rr c
	pop af
	jr z, ssf_whole
	inc bc
ssf_whole:
	push hl
	ld hl, STORE_HEADER
	add hl, bc
	ld b, h
	ld c, l
	pop hl
	ret

; In: DE = assertion number, HL = header, BC = count. Out: A = 0 and HL = the table byte that
; holds the number and C = how far its 2 bits are shifted, 0, 2, 4 or 6; or A = ST_RANGE.
store_locate:
	push hl
	ld h, d
	ld l, e
	or a
	sbc hl, bc
	pop hl
	jr c, sl_in
	ld a, ST_RANGE
	ret
sl_in:
	ld a, e
	and 3
	add a, a
	ld c, a
	srl d
	rr e
	srl d
	rr e
	add hl, de
	ld de, STORE_HEADER
	add hl, de
	xor a
	ret

store_get:
	push hl
	call store_data
	pop de
	or a
	ret nz
	call store_locate
	or a
	ret nz
	ld a, (hl)
	ld b, c
	inc b
	jr sg_first
sg_shift:
	rrca
sg_first:
	djnz sg_shift
	and 3
	ld b, a
	xor a
	ret

store_set:
	ld a, b
	cp 4
	ld a, ST_RANGE
	ret nc
	push bc
	push hl
	call store_data
	pop de
	or a
	jr nz, ss_fail
	call store_locate
	or a
	jr nz, ss_fail
	pop de
	ld e, 3
	ld b, c
	inc b
	jr ss_first
ss_shift:
	sla e
	sla d
ss_first:
	djnz ss_shift
	ld a, e
	cpl
	and (hl)
	or d
	ld (hl), a
	xor a
	ret
ss_fail:
	pop bc
	ret

store_get_next:
	call store_data
	or a
	ret nz
	ld de, STORE_OFF_NEXT
	add hl, de
	ld e, (hl)
	inc hl
	ld d, (hl)
	ex de, hl
	xor a
	ret

store_set_next:
	push hl
	call store_data
	pop de
	or a
	ret nz
	push hl
	ld h, b
	ld l, c
	or a
	sbc hl, de
	pop hl
	jr nc, ssn_in
	ld a, ST_RANGE
	ret
ssn_in:
	ld bc, STORE_OFF_NEXT
	add hl, bc
	ld (hl), e
	inc hl
	ld (hl), d
	xor a
	ret

store_reset:
	call store_data
	or a
	ret nz
	push hl
	call store_size_for
	ld hl, -STORE_OFF_NEXT
	add hl, bc
	ld b, h
	ld c, l
	pop hl
	ld de, STORE_OFF_NEXT
	add hl, de
	ld (hl), 0
	ld d, h
	ld e, l
	inc de
	dec bc
	ldir
	xor a
	ret

; Counting runs in blocks of at most 63 table bytes, so one 8-bit counter per value, in OP2,
; cannot overflow, and adds the counters to the caller's words after each block.
CNT_BUF   = OP3
CNT_SRC   = OP3 + 2
CNT_LEFT  = OP3 + 4
CNT_TAIL  = OP3 + 6
CNT_BLOCK = 63
CNT_C0    = OP2

store_count:
	push hl
	call store_data
	pop de
	or a
	ret nz
	ld (CNT_BUF), de
	ld a, c
	and 3
	ld (CNT_TAIL), a
	srl b
	rr c
	srl b
	rr c
	ld (CNT_LEFT), bc
	ld de, STORE_HEADER
	add hl, de
	ld (CNT_SRC), hl
	ld hl, (CNT_BUF)
	ld d, h
	ld e, l
	inc de
	ld (hl), 0
	ld bc, 7
	ldir
	ld hl, CNT_C0
	ld d, h
	ld e, l
	inc de
	ld (hl), 0
	ld bc, 3
	ldir
sc_block:
	ld bc, (CNT_LEFT)
	ld a, b
	or c
	jr z, sc_tail
	ld a, b
	or a
	jr nz, sc_full
	ld a, c
	cp CNT_BLOCK
	jr c, sc_part
sc_full:
	ld a, CNT_BLOCK
sc_part:
	ld e, a
	ld d, 0
	ld hl, (CNT_LEFT)
	or a
	sbc hl, de
	ld (CNT_LEFT), hl
	ld b, a
	ld de, (CNT_SRC)
	ld h, CNT_C0 >> 8
sc_byte:
	ld a, (de)
	inc de
	ld c, a
	and 3
	add a, CNT_C0 & 0FFh
	ld l, a
	inc (hl)
	ld a, c
	rrca
	rrca
	ld c, a
	and 3
	add a, CNT_C0 & 0FFh
	ld l, a
	inc (hl)
	ld a, c
	rrca
	rrca
	ld c, a
	and 3
	add a, CNT_C0 & 0FFh
	ld l, a
	inc (hl)
	ld a, c
	rrca
	rrca
	and 3
	add a, CNT_C0 & 0FFh
	ld l, a
	inc (hl)
	djnz sc_byte
	ld (CNT_SRC), de
	call sc_flush
	jr sc_block
sc_tail:
	ld a, (CNT_TAIL)
	or a
	jr z, sc_done
	ld b, a
	ld de, (CNT_SRC)
	ld a, (de)
	ld c, a
	ld h, CNT_C0 >> 8
sc_field:
	ld a, c
	and 3
	add a, CNT_C0 & 0FFh
	ld l, a
	inc (hl)
	srl c
	srl c
	djnz sc_field
	call sc_flush
sc_done:
	xor a
	ret

; Adds the four block counters to the words of the caller's buffer and clears them.
sc_flush:
	ld hl, (CNT_BUF)
	ld de, CNT_C0
	ld b, 4
sc_flush_one:
	ld a, (de)
	add a, (hl)
	ld (hl), a
	inc hl
	jr nc, sc_flush_low
	inc (hl)
sc_flush_low:
	inc hl
	xor a
	ld (de), a
	inc de
	djnz sc_flush_one
	ret

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
