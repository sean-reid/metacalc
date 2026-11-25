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
; a variable, so every routine but store_open and store_has finds MCSTAT again through
; store_data, the one routine that returns the data pointer, and uses that pointer only until its
; next OS call; store_open and store_has call _ChkFindSym themselves because they have to deal
; with a variable that is absent, archived or not this library's, and store_has returns no
; pointer. Callers hold no pointer into the table either. Each routine but store_open checks the
; size, magic and version it finds.
;
; Status, in A, for every routine: 0 is ok, and ST_OK is the kernel's name for it. The others:
ST_NO_RAM        = 1    ; open: not enough RAM for the table, nothing created
ST_ARCHIVED      = 2    ; the variable is archived and could not be unarchived
ST_BAD_MAGIC     = 3    ; wrong magic or version
ST_OTHER_LIBRARY = 4    ; another library's CRC or another assertion count
ST_BAD_SIZE      = 5    ; the size is not what the header implies
ST_RANGE         = 6    ; an assertion number, a value, a next number or a count end out of range
ST_ABSENT        = 7    ; no variable: open was not called, or the user deleted it
;
; All routines destroy every register but IY, which must be the OS flags pointer, and the
; stack. They destroy OP1 and OP2, and the count routines also OP3 to OP5.
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
; store_get_next  out: A = status, HL = number of the next assertion to check; ST_RANGE when
;              the header holds a number above the count
; store_set_next  in:  HL = number, at most the count    out: A = status
; store_count  in:  HL = 8 bytes outside the table and OP1 to OP5
;              out: A = status; the buffer holds four little-endian words, n0 to n3, the number
;                   of assertions with value 0, 1, 2 and 3. The model's totals over them:
;                   checked = n1 + n3, failed = n2, settled = n1 + n2 + n3, total = the count.
; store_count_to  in:  HL = the buffer, DE = N    out: A = status; n0 to n3 over the first N
;              assertions only, a last partial byte counted up to N. ST_RANGE when N is above
;              the count, with the buffer untouched.
; store_count_ends  in:  HL = a list, DE = results; both outside the table and OP1 to OP5, and
;              the list outside the results; the list is a count byte K, 1 to 8, then K
;              little-endian words, the ends, of which the first may be 0
;              out: A = status; K buffers of 8 bytes, the end's n0 to n3 over the first `end`
;              assertions, in one pass over the table. The ends must rise strictly and the last
;              must not exceed the count; otherwise ST_RANGE, with the results untouched.
;              store_count is the list of the count, store_count_to a list of N.
; store_reset  out: A = status; every assertion unchecked and the next number 0
; store_has    in:  DE:HL = the library's CRC-32 (DE the high 16 bits), BC = assertion count
;              out: A = 0 when MCSTAT is in RAM with the right magic, version and size and this
;                   CRC and count; else ST_ABSENT, ST_ARCHIVED, ST_BAD_MAGIC, ST_OTHER_LIBRARY or
;                   ST_BAD_SIZE as store_open would find it. It writes nothing and makes no OS
;                   call but _ChkFindSym, so it never creates, deletes or unarchives anything.
; store_all_ones  in:  HL = a list of BC 16-bit little-endian entries outside the table and OP1
;              to OP3, BC = the entry count, 0 allowed
;              out: A = status; with 0, B = 1 when every entry with bit 15 clear names an
;                   assertion of value 1 and B = 0 at the first that does not, the rest unread.
;                   Entries with bit 15 set are skipped, a dummy variable in a reference list.
;                   An entry with bit 15 clear that is not below the count gives ST_RANGE and
;                   B = 0. The table is found once and no OS call follows, so the list is read
;                   against one table.
; store_next_zero  in:  HL = from, DE = end, from at most end and end at most the count
;              out: A = status; HL = the number of the first assertion from `from` below `end`
;                   whose value is 0, or `end` when there is none. ST_RANGE otherwise. Whole table
;                   bytes with no 0 in them are passed four assertions at a time. It destroys OP1
;                   to OP3.
; store_next_not_one  in:  HL = from, DE = end, from at most end and end at most the count
;              out: A = status; HL = the number of the first assertion from `from` below `end`
;                   whose value is not 1, or `end` when there is none. ST_RANGE otherwise. Whole
;                   table bytes of four 1s are passed four assertions at a time. It destroys OP1
;                   to OP3.
; store_prev_not_one  in:  HL = from, at most the count
;              out: A = status; HL = the number of the last assertion below `from` whose value is
;                   not 1, or FFFFh when there is none. ST_RANGE for a from above the count. Whole
;                   table bytes of four 1s are passed four assertions at a time, going down. It
;                   destroys OP1 to OP3.
; Padding: the unused bits of the last table byte are never read. store_set changes only the two
; bits it is given and leaves the padding as it is; store_reset and a newly
; created table zero it; store_count ignores it.
OPEN_WAS_ARCHIVED = 80h

#ifndef STORE_VAR
#define STORE_VAR "MCSTAT"
#endif
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
	.db appVarObj, STORE_VAR, 0, 0

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
	call store_unpack
	ld b, a
	xor a
	ret

; In: HL = the table byte store_locate gave, C = its shift. Out: A = the value, 0 to 3. Destroys B.
store_unpack:
	ld a, (hl)
	ld b, c
	inc b
	jr su_first
su_shift:
	rrca
su_first:
	djnz su_shift
	and 3
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
	ld h, b
	ld l, c
	or a
	sbc hl, de
	jr c, sgn_range
	ex de, hl
	xor a
	ret
sgn_range:
	ld a, ST_RANGE
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

#include "store_count.inc"
#include "store_marks.inc"
#include "store_open.inc"
