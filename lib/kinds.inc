; The kinds section: one bit per assertion, set for a theorem. See lib.z80 for the contract.

lk_end              .equ lr_cn
lk_from             .equ lr_q
lk_p                .equ lr_n
lk_last             .equ lr_sr
lk_kind             .equ lr_k
lk_mask             .equ lr_tc
lk_cnt              .equ lr_cnt
lk_lim              .equ lr_len
LK_FIRST_RUN        .equ 8

; in: lk_kind = the caller's kind. out: Z and lk_kind = the byte the bitmap is xored with so that
; a set bit is a match, or NZ and A = status. Destroys all but IX, IY.
lk_begin:
	call lr_need
	ret nz
	ld a,(lk_kind)
	cp 2
	jp nc,lr_bad_arg
	dec a
	ld (lk_kind),a
	ld a,(ls_have)
	and LS_KINDS
	jp z,lsx_nothing
	xor a
	ret

; in: lk_p = the first byte of the bitmap to read, BC = how many (1 to LIB_WINDOW). Reads them into
; lr_win and drops the record window, which that buffer holds. out: Z, or NZ and A = status.
; Destroys all but IX, IY.
lk_read:
	xor a
	ld (lr_wlen),a
	ld (lr_wp),a
	push bc
	ld hl,(lk_p)
	ld de,ls_kd
	call lsx_add3
	ld bc,2
	call lr_adv
	pop bc
	ld de,lr_win
	jp lr_rd

; in: B = kind (0 axiom, 1 theorem), HL = from, DE = end
; out: A = status; HL = the first assertion of that kind from from to below end, else end.
lib_kind_next:
	ld (lk_from),hl
	ld (lk_end),de
	ld a,b
	ld (lk_kind),a
	call lk_begin
	ret nz
	ld hl,(lr_count)
	ld de,(lk_end)
	or a
	sbc hl,de
	jp c,lr_bad_index
	ld hl,(lk_end)
	ld de,(lk_from)
	or a
	sbc hl,de
	jp c,lr_bad_index
	jr nz,lkn_scan
	ld hl,(lk_end)
	xor a
	ret
lkn_scan:
	ld hl,(lk_end)
	dec hl
	srl h
	rr l
	srl h
	rr l
	srl h
	rr l
	ld (lk_last),hl
	ld hl,(lk_from)
	ld a,l
	and 7
	ld b,a
	ld a,$FF
	inc b
	jr lkn_shift_in
lkn_shift:
	add a,a
lkn_shift_in:
	djnz lkn_shift
	ld (lk_mask),a
	srl h
	rr l
	srl h
	rr l
	srl h
	rr l
	ld (lk_p),hl
	ld a,LK_FIRST_RUN
	ld (lk_lim),a
lkn_chunk:
	ld hl,(lk_last)
	ld de,(lk_p)
	or a
	sbc hl,de
	ld a,(lk_lim)
	ld b,a
	ld a,h
	or a
	jr nz,lkn_full
	ld a,b
	dec a
	cp l
	jr c,lkn_full
	jr z,lkn_full
	ld a,l
	inc a
	jr lkn_counted
lkn_full:
	ld a,b
lkn_counted:
	ld (lk_cnt),a
	ld c,a
	ld b,0
	call lk_read
	ret nz
	ld a,(lk_cnt)
	ld b,a
	ld a,(lk_mask)
	ld c,a
	ld a,(lk_kind)
	ld d,a
	ld hl,lr_win
lkn_byte:
	ld a,(hl)
	xor d
	and c
	jr nz,lkn_found
	ld c,$FF
	inc hl
	djnz lkn_byte
	ld a,c
	ld (lk_mask),a
	ld a,LIB_WINDOW
	ld (lk_lim),a
	ld hl,(lk_p)
	ld a,(lk_cnt)
	ld e,a
	ld d,0
	add hl,de
	ld (lk_p),hl
	ld de,(lk_last)
	ex de,hl
	or a
	sbc hl,de
	jr nc,lkn_chunk
lkn_none:
	ld hl,(lk_end)
	xor a
	ret
lkn_found:
	ld c,0
lkn_low:
	srl a
	jr c,lkn_bit
	inc c
	jr lkn_low
lkn_bit:
	ld de,lr_win
	or a
	sbc hl,de
	ld de,(lk_p)
	add hl,de
	add hl,hl
	add hl,hl
	add hl,hl
	ld b,0
	add hl,bc
	ld de,(lk_end)
	or a
	sbc hl,de
	jr c,lkn_hit
	ex de,hl
	xor a
	ret
lkn_hit:
	add hl,de
	xor a
	ret

; in: B = kind (0 axiom, 1 theorem), HL = from
; out: A = status; HL = the last assertion of that kind below from, else FFFFh.
lib_kind_prev:
	ld (lk_from),hl
	ld a,b
	ld (lk_kind),a
	call lk_begin
	ret nz
	ld hl,(lr_count)
	ld de,(lk_from)
	or a
	sbc hl,de
	jp c,lr_bad_index
	ld hl,(lk_from)
	ld a,h
	or l
	jr nz,lkp_scan
	ld hl,$FFFF
	ret
lkp_scan:
	dec hl
	ld a,l
	and 7
	ld b,a
	ld a,7
	sub b
	ld b,a
	ld a,$FF
	inc b
	jr lkp_shift_in
lkp_shift:
	srl a
lkp_shift_in:
	djnz lkp_shift
	ld (lk_mask),a
	srl h
	rr l
	srl h
	rr l
	srl h
	rr l
	ld (lk_last),hl
	ld a,LK_FIRST_RUN
	ld (lk_lim),a
lkp_chunk:
	ld hl,(lk_last)
	ld a,(lk_lim)
	ld b,a
	ld a,h
	or a
	jr nz,lkp_full
	ld a,b
	dec a
	cp l
	jr c,lkp_full
	jr z,lkp_full
	ld a,l
	inc a
	jr lkp_counted
lkp_full:
	ld a,b
lkp_counted:
	ld (lk_cnt),a
	ld c,a
	ld b,0
	ld hl,(lk_last)
	or a
	sbc hl,bc
	inc hl
	ld (lk_p),hl
	call lk_read
	ret nz
	ld a,(lk_cnt)
	ld b,a
	ld e,a
	ld d,0
	ld hl,lr_win-1
	add hl,de
	ld a,(lk_mask)
	ld c,a
	ld a,(lk_kind)
	ld d,a
lkp_byte:
	ld a,(hl)
	xor d
	and c
	jr nz,lkp_found
	ld c,$FF
	dec hl
	djnz lkp_byte
	ld a,c
	ld (lk_mask),a
	ld a,LIB_WINDOW
	ld (lk_lim),a
	ld hl,(lk_p)
	ld a,h
	or l
	jr z,lkp_none
	dec hl
	ld (lk_last),hl
	jr lkp_chunk
lkp_none:
	ld hl,$FFFF
	xor a
	ret
lkp_found:
	ld c,7
lkp_high:
	add a,a
	jr c,lkp_bit
	dec c
	jr lkp_high
lkp_bit:
	ld de,lr_win
	or a
	sbc hl,de
	ld de,(lk_p)
	add hl,de
	add hl,hl
	add hl,hl
	add hl,hl
	ld b,0
	add hl,bc
	xor a
	ret
