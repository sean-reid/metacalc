; Record parsing and the rules every record must keep.

; in: HL at the cursor. out: A = byte, HL advanced. Refuses a read past the record's end. Destroys DE.
pr_u8:
	ld de,(kv_rec_end)
	push hl
	or a
	sbc hl,de
	pop hl
	jp nc,fail_malformed
	ld a,(hl)
	inc hl
	ret

; in: HL at the cursor. out: BC = word, HL advanced. Destroys AF DE.
pr_u16:
	call pr_u8
	ld c,a
	call pr_u8
	ld b,a
	ret

; in: HL at the cursor, BC = count. out: HL advanced; refused past the record's end. Destroys DE.
pr_skip:
	add hl,bc
	jp c,fail_malformed
	ld de,(kv_rec_end)
	ex de,hl
	or a
	sbc hl,de
	jp c,fail_malformed
	ex de,hl
	ret

; in: BC = count. out: BC = 2 * count, refused if it needs more than 16 bits.
pr_double:
	sla c
	rl b
	ret nc
	jp fail_malformed

; in: HL = assertion number, IX = descriptor.
; out: record parsed into the descriptor and its page mapped. Refuses a record that leaves
; the library, its page, or the layout. Destroys AF BC DE HL.
parse_record:
	ld (ix+D_ID),l
	ld (ix+D_ID+1),h
	ld de,(kv_assertions)
	or a
	sbc hl,de
	jp nc,fail_malformed
	add hl,de
	ld e,0
	add hl,hl
	rl e
	add hl,hl
	rl e
	ld bc,(kv_directory)
	add hl,bc
	ld a,(kv_directory+2)
	adc a,e
	ld e,a
	call map_offset
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld c,(hl)
	inc hl
	ld a,(hl)
	or a
	jp nz,fail_malformed
	ld (kv_tmp24),de
	ld a,c
	ld (kv_tmp24+2),a
	ld hl,kv_tmp24
	ld de,kv_length
	call cmp24
	jp nc,fail_malformed
	ld hl,(kv_tmp24)
	ld a,(kv_tmp24+2)
	ld e,a
	call map_offset
	ld (ix+D_PAGE),c
	ld a,(kv_len_chunk)
	cp b
	ld de,$C000
	jr nz,parse_bound
	ld de,(kv_len_tail)
parse_bound:
	ld (kv_rec_end),de

	call pr_u8
	cp 2
	jp nc,fail_malformed
	ld (ix+D_KIND),a
	call pr_u8
	ld c,a
	ld b,0
	call pr_skip
	call pr_u8
	ld (ix+D_NHYPS),a
	ld (ix+D_HYPS),l
	ld (ix+D_HYPS+1),h
	or a
	jr z,parse_hyps_done
	ld b,a
parse_hyp:
	push bc
	call pr_u8
	or a
	jr z,parse_float
	cp 1
	jp nz,fail_malformed
	call pr_u8
	ld c,a
	ld b,0
	jr parse_hyp_skip
parse_float:
	ld bc,1
parse_hyp_skip:
	call pr_skip
	pop bc
	djnz parse_hyp
parse_hyps_done:
	call pr_u16
	ld (ix+D_NDV),c
	ld (ix+D_NDV+1),b
	ld (ix+D_DV),l
	ld (ix+D_DV+1),h
	call pr_double
	call pr_skip
	call pr_u8
	ld (ix+D_NCONC),a
	ld (ix+D_CONC),l
	ld (ix+D_CONC+1),h
	ld c,a
	ld b,0
	call pr_skip
	xor a
	ld (ix+D_NEXTRA),a
	ld (ix+D_NEXTRA+1),a
	ld (ix+D_NREFS),a
	ld (ix+D_NREFS+1),a
	ld (ix+D_PLEN),a
	ld (ix+D_PLEN+1),a
	ld a,(ix+D_KIND)
	or a
	ret z
	call pr_u16
	ld (ix+D_NEXTRA),c
	ld (ix+D_NEXTRA+1),b
	ld (ix+D_EXTRA),l
	ld (ix+D_EXTRA+1),h
	call pr_double
	call pr_skip
	call pr_u16
	ld (ix+D_NREFS),c
	ld (ix+D_NREFS+1),b
	ld (ix+D_REFS),l
	ld (ix+D_REFS+1),h
	call pr_double
	call pr_skip
	call pr_u16
	ld (ix+D_PLEN),c
	ld (ix+D_PLEN+1),b
	ld (ix+D_PROOF),l
	ld (ix+D_PROOF+1),h
	jp pr_skip

; in: IX = descriptor of the record just parsed, its page mapped.
; out: returns if the record passes check_record, using and filling the cache.
require_sound:
	ld l,(ix+D_ID)
	ld h,(ix+D_ID+1)
	ld a,h
	cp CACHE_IDS>>8
	jp nc,check_record
	ld a,l
	and 7
	ld b,a
	ld a,1
	inc b
rs_mask:
	dec b
	jr z,rs_masked
	add a,a
	jr rs_mask
rs_masked:
	ld c,a
	srl h
	rr l
	srl h
	rr l
	srl h
	rr l
	ld de,CACHE
	add hl,de
	ld a,(hl)
	and c
	ret nz
	push hl
	push bc
	call check_record
	pop bc
	pop hl
	ld a,(hl)
	or c
	ld (hl),a
	ret

; in: IX = descriptor, its page mapped. out: returns if the record keeps the rules.
check_record:
	ld hl,FLOATS
	ld bc,256
	xor a
	call fill_bytes
	ld a,(ix+D_NHYPS)
	or a
	jr z,cr_conclusion
	ld b,a
	ld l,(ix+D_HYPS)
	ld h,(ix+D_HYPS+1)
	ld e,0
cr_hyp:
	ld a,(hl)
	inc hl
	or a
	jr z,cr_float
	ld e,1
	ld c,(hl)
	inc hl
	push hl
	push bc
	call check_expr
	pop bc
	pop hl
	ld a,c
	call add_hl_a
	jr cr_hyp_next
cr_float:
	ld a,e
	or a
	jp nz,fail_bad_record
	ld a,(hl)
	inc hl
	push hl
	ld h,TYPECODES>>8
	ld l,a
	ld c,(hl)
	inc c
	jp z,fail_bad_record
	ld h,FLOATS>>8
	ld a,(hl)
	or a
	jp nz,fail_bad_record
	ld (hl),1
	pop hl
cr_hyp_next:
	djnz cr_hyp
cr_conclusion:
	ld l,(ix+D_CONC)
	ld h,(ix+D_CONC+1)
	ld c,(ix+D_NCONC)
	call check_expr
	ld a,(ix+D_NHYPS)
	or a
	jr z,cr_used_done
	ld b,a
	ld l,(ix+D_HYPS)
	ld h,(ix+D_HYPS+1)
cr_used:
	ld a,(hl)
	inc hl
	or a
	jr z,cr_used_float
	ld a,(hl)
	inc hl
	call add_hl_a
	jr cr_used_next
cr_used_float:
	ld a,(hl)
	inc hl
	push hl
	ld h,FLOATS>>8
	ld l,a
	bit 1,(hl)
	jp z,fail_bad_record
	pop hl
cr_used_next:
	djnz cr_used
cr_used_done:
	xor a
	ld (kv_mode),a
	ld l,(ix+D_DV)
	ld h,(ix+D_DV+1)
	ld c,(ix+D_NDV)
	ld b,(ix+D_NDV+1)
	call check_pairs
	ld a,1
	ld (kv_mode),a
	ld l,(ix+D_EXTRA)
	ld h,(ix+D_EXTRA+1)
	ld c,(ix+D_NEXTRA)
	ld b,(ix+D_NEXTRA+1)
	jp check_pairs

; in: HL = expression, C = length. out: returns if it is not empty, starts with a constant and
; uses only constants and variables with a float mark, whose bit 1 it sets as used.
check_expr:
	ld a,c
	or a
	jp z,fail_bad_record
	ld a,(kv_constants)
	ld d,a
	ld a,(hl)
	cp d
	jp nc,fail_bad_record
	ld b,c
ce_symbol:
	ld a,(hl)
	inc hl
	cp d
	jr c,ce_known
	push hl
	ld h,FLOATS>>8
	ld l,a
	ld a,(hl)
	or a
	jp z,fail_bad_record
	set 1,(hl)
	pop hl
ce_known:
	djnz ce_symbol
	ret

; in: HL = pairs, BC = count, kv_mode = 0 for mandatory pairs, 1 for the extra ones.
; out: returns if each pair names two different variables, and both have a float mark
; exactly when kv_mode is 0.
check_pairs:
cp_next:
	ld a,b
	or c
	ret z
	dec bc
	ld d,(hl)
	inc hl
	ld e,(hl)
	inc hl
	ld a,d
	cp e
	jp z,fail_bad_record
	push hl
	ld h,TYPECODES>>8
	ld l,d
	ld a,(hl)
	inc a
	jp z,fail_bad_record
	ld l,e
	ld a,(hl)
	inc a
	jp z,fail_bad_record
	ld h,FLOATS>>8
	ld l,d
	ld a,(hl)
	ld l,e
	and (hl)
	and 1
	pop hl
	ld d,a
	ld a,(kv_mode)
	xor d
	cp 1
	jp nz,fail_bad_record
	jr cp_next
