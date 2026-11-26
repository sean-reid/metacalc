; The label order section: every assertion number once, sorted by label. See lib.z80 for the
; contract. Entry i of the section is at ls_od + 2 + 2 * i.

fr_ptr              .equ lr_bufp
fr_len              .equ lr_len
fr_cnt              .equ lr_cn
fr_lo               .equ lr_n
fr_hi               .equ lr_sr
fr_mid              .equ lr_q
fr_c                .equ lr_k
fr_first            .equ lr_tr

wk_idx              .equ lr_dc
wk_rem              .equ lr_dc+2
wk_start            .equ lr_dc+4
wk_bound            .equ lr_dc+6
wk_kind             .equ lr_dc+8
wk_best             .equ lr_dc+9
wk_r                .equ lr_dc+11
wk_p                .equ lr_dc+12
WK_BLOCK            .equ 9
WK_RUN              .equ 16
FC_RUN              .equ 127

; out: Z, or NZ and A = status: 4 before lib_open, 9 without the section. Destroys all but IX, IY.
fo_begin:
	call lr_need
	ret nz
	ld a,(ls_have)
	and LS_ORDER
	jp z,lsx_nothing
	xor a
	ret

; in: HL = section index. out: Z and HL = the assertion number there, or NZ and A = status.
; Reads the entry into lr_hdr. Destroys all but IX, IY.
fo_number:
	xor a
	add hl,hl
	rla
	ld bc,2
	call lr_adv
	ld de,ls_od
	call lsx_add3
	ld de,lr_hdr
	ld bc,2
	call lr_rd
	ret nz
	ld hl,(lr_hdr)
	ld bc,(lr_count)
	or a
	sbc hl,bc
	jp nc,lr_bad
	add hl,bc
	xor a
	ret

; in: HL = assertion number. out: Z, or NZ and A = status. Reads the label's length and the first
; fr_cnt - 1 bytes after it into lr_win. Destroys all but IX, IY.
fo_label:
	xor a
	add hl,hl
	rla
	add hl,hl
	rla
	ld de,lr_dir
	call lsx_add3
	call lsx_entry4
	ret nz
	ld bc,1
	call lr_adv
	ld de,lr_win
	ld bc,(fr_cnt)
	jp lr_rd

; in: lr_win = a label's length byte and bytes, fr_ptr and fr_len = the prefix. out: Z and B = 0
; when the label starts with the prefix, 1 when it sorts after, FFh when it sorts before: the
; label's first bytes up to the shorter length are compared with the prefix's, and a label that
; is shorter than the prefix and equal to its start sorts before. Destroys all but IX, IY.
fo_cmp:
	ld a,(lr_win)
	ld c,a
	ld a,(fr_len)
	ld b,a
	ld a,c
	cp b
	jr nc,foc_len
	ld b,a
foc_len:
	ld a,b
	or a
	jr z,foc_same
	ld hl,lr_win+1
	ld de,(fr_ptr)
foc_byte:
	ld a,(de)
	cp (hl)
	jr nz,foc_differ
	inc hl
	inc de
	djnz foc_byte
foc_same:
	ld a,(fr_len)
	ld b,0
	cp c
	jr z,foc_done
	jr c,foc_done
	dec b
foc_done:
	xor a
	ret
foc_differ:
	ld b,1
	jr c,foc_done
	dec b
	dec b
	jr foc_done

; in: fr_mid = a section index. out: Z and B = fo_cmp's answer for its label, or NZ and A = status.
; Destroys all but IX, IY.
fo_probe:
	ld hl,(fr_mid)
	call fo_number
	ret nz
	call fo_label
	ret nz
	jr fo_cmp

; in: DE = prefix, B = its length, 1 to 8
; out: A = status; HL = the index in the section of the first entry whose label starts with the
; prefix, BC = how many consecutive entries do. BC = 0 when none does, and HL is then the index
; where the prefix would sort: the first entry whose label sorts after it, or the assertion count.
lib_find_range:
	ld (fr_ptr),de
	ld a,b
	ld (fr_len),a
	call fo_begin
	ret nz
	ld a,(fr_len)
	or a
	jp z,lr_bad_arg
	cp 9
	jp nc,lr_bad_arg
	ld c,a
	ld b,0
	ld de,(fr_ptr)
	call lr_chkbuf
	ret nz
	ld a,c
	inc a
	ld (fr_cnt),a
	xor a
	ld (fr_cnt+1),a
	ld (lr_wlen),a
	ld (lr_wp),a
	ld h,a
	ld l,a
	ld (fr_lo),hl
	ld hl,(lr_count)
	ld (fr_hi),hl
frl_lower:
	ld hl,(fr_hi)
	ld de,(fr_lo)
	or a
	sbc hl,de
	jr z,frl_found
	srl h
	rr l
	add hl,de
	ld (fr_mid),hl
	call fo_probe
	ret nz
	ld a,b
	inc a
	jr z,frl_before
	dec a
	ld (fr_c),a
	ld hl,(fr_mid)
	ld (fr_hi),hl
	jr frl_lower
frl_before:
	ld hl,(fr_mid)
	inc hl
	ld (fr_lo),hl
	jr frl_lower
frl_found:
	ld hl,(lr_count)
	or a
	sbc hl,de
	jr z,frl_none
	ld a,(fr_c)
	or a
	jr z,frl_upper
frl_none:
	ld hl,(fr_lo)
	ld bc,0
	xor a
	ret
frl_upper:
	ld hl,(fr_lo)
	ld (fr_first),hl
	inc hl
	ld (fr_lo),hl
	ld hl,(lr_count)
	ld (fr_hi),hl
frl_search:
	ld hl,(fr_hi)
	ld de,(fr_lo)
	or a
	sbc hl,de
	jr z,frl_end
	srl h
	rr l
	add hl,de
	ld (fr_mid),hl
	call fo_probe
	ret nz
	ld a,b
	or a
	jr nz,frl_after
	ld hl,(fr_mid)
	inc hl
	ld (fr_lo),hl
	jr frl_search
frl_after:
	ld hl,(fr_mid)
	ld (fr_hi),hl
	jr frl_search
frl_end:
	ld hl,(fr_lo)
	ld de,(fr_first)
	or a
	sbc hl,de
	ld b,h
	ld c,l
	ex de,hl
	xor a
	ret

; in: DE = an assertion number below the assertion count. out: Z and B = its bit of the kinds
; bitmap, or NZ and A = status. Destroys all but IX, IY.
fwk_bit:
	ld h,d
	ld l,e
	srl h
	rr l
	srl h
	rr l
	srl h
	rr l
	push de
	xor a
	ld bc,2
	call lr_adv
	ld de,ls_kd
	call lsx_add3
	ld de,lr_hdr
	ld bc,1
	call lr_rd
	pop de
	ret nz
	ld a,e
	and 7
	inc a
	ld b,a
	ld a,(lr_hdr)
fwkb_shift:
	rrca
	djnz fwkb_shift
	ld a,0
	adc a,0
	ld b,a
	xor a
	ret

; in: HL = a block of 9 bytes in bank B: first index (u16) and count of a range, start (u16), bound
; (u16), kind (u8: 0 axiom, 1 theorem, FFh every assertion)
; out: A = status; HL = the smallest assertion number n among the entries first to first + count - 1
; of the section with start <= n < bound that the kind shows, or the bound when there is none.
lib_find_walk:
	ld (fr_ptr),hl
	call fo_begin
	ret nz
	ld bc,WK_BLOCK
	ld de,(fr_ptr)
	call lr_chkbuf
	ret nz
	ld hl,(fr_ptr)
	ld de,WK_BLOCK-1
	add hl,de
	ld a,(hl)
	inc a
	jr z,fwk_kind_ok
	dec a
	cp 2
	jp nc,lr_bad_arg
	ld a,(ls_have)
	and LS_KINDS
	jp z,lsx_nothing
fwk_kind_ok:
	ld hl,(fr_ptr)
	ld de,wk_idx
	ld bc,WK_BLOCK
	ldir
	ld hl,$FFFF
	ld (lr_dcbase),hl
	xor a
	ld (lr_wlen),a
	ld (lr_wp),a
	ld hl,(wk_idx)
	ld de,(wk_rem)
	add hl,de
	jp c,lr_bad_index
	ld de,(lr_count)
	ex de,hl
	or a
	sbc hl,de
	jp c,lr_bad_index
	ld hl,(lr_count)
	ld de,(wk_bound)
	or a
	sbc hl,de
	jp c,lr_bad_index
	ld hl,(wk_bound)
	ld de,(wk_start)
	or a
	sbc hl,de
	jp c,lr_bad_index
	ld hl,(wk_bound)
	ld (wk_best),hl
fwk_run:
	ld hl,(wk_rem)
	ld a,h
	or l
	jp z,fwk_done
	ld a,h
	or a
	ld a,WK_RUN
	jr nz,fwk_take
	ld a,l
	cp WK_RUN
	jr c,fwk_take
	ld a,WK_RUN
fwk_take:
	ld (wk_r),a
	ld e,a
	ld d,0
	ld hl,(wk_rem)
	or a
	sbc hl,de
	ld (wk_rem),hl
	ld hl,(wk_idx)
	xor a
	add hl,hl
	rla
	ld bc,2
	call lr_adv
	ld de,ls_od
	call lsx_add3
	ld b,a
	ld a,(wk_r)
	add a,a
	ld c,a
	ld a,b
	ld b,0
	ld de,lr_win
	call lr_rd
	ret nz
	ld a,(wk_r)
	ld e,a
	ld d,0
	ld hl,(wk_idx)
	add hl,de
	ld (wk_idx),hl
	xor a
	ld (wk_p),a
fwk_entry:
	ld a,(wk_p)
	ld l,a
	ld h,0
	ld de,lr_win
	add hl,de
	ld e,(hl)
	inc hl
	ld d,(hl)
	ld hl,(wk_start)
	or a
	sbc hl,de
	jr z,fwk_in
	jr nc,fwk_next
fwk_in:
	ld hl,(wk_best)
	or a
	sbc hl,de
	jr z,fwk_next
	jr c,fwk_next
	ld a,(wk_kind)
	inc a
	jr z,fwk_take_n
	call fwk_bit
	ret nz
	ld a,(wk_kind)
	cp b
	jr nz,fwk_next
fwk_take_n:
	ld (wk_best),de
fwk_next:
	ld hl,wk_p
	inc (hl)
	inc (hl)
	ld hl,wk_r
	dec (hl)
	jr nz,fwk_entry
	jp fwk_run
fwk_done:
	ld hl,(wk_best)
	xor a
	ret

; in: HL = the first index of a range in the section, BC = its count, DE = buffer in bank B
; out: A = status; HL = how many entries were copied, the smaller of the count and 127, to the
; buffer as stored: two bytes each, the assertion number little endian, the entries first to first + HL - 1.
lib_find_copy:
	ld (fr_first),hl
	ld (fr_ptr),de
	ld (fr_cnt),bc
	call fo_begin
	ret nz
	ld hl,(fr_cnt)
	ld a,h
	or a
	ld a,FC_RUN
	jr nz,fcp_take
	ld a,l
	cp FC_RUN
	jr c,fcp_take
	ld a,FC_RUN
fcp_take:
	ld (fr_len),a
	or a
	jr z,fcp_range
	add a,a
	ld c,a
	ld b,0
	ld de,(fr_ptr)
	call lr_chkbuf
	ret nz
fcp_range:
	ld hl,(fr_first)
	ld de,(fr_cnt)
	add hl,de
	jp c,lr_bad_index
	ld de,(lr_count)
	ex de,hl
	or a
	sbc hl,de
	jp c,lr_bad_index
	ld a,(fr_len)
	or a
	jr z,fcp_none
	ld hl,(fr_first)
	xor a
	add hl,hl
	rla
	ld bc,2
	call lr_adv
	ld de,ls_od
	call lsx_add3
	ld b,a
	ld a,(fr_len)
	add a,a
	ld c,a
	ld a,b
	ld b,0
	ld de,(fr_ptr)
	call lr_rd
	ret nz
fcp_none:
	ld a,(fr_len)
	ld l,a
	ld h,0
	xor a
	ret
