; Applying an assertion to the stack.

; in: A = variable. out: HL = start and DE = end of the stack range bound to it by the apply in
; progress; BadRecord if it is not bound. Destroys AF BC.
subst_get:
	ld l,a
	ld h,0
	add hl,hl
	add hl,hl
	add hl,hl
	ld de,SUBST
	add hl,de
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld c,(hl)
	inc hl
	ld b,(hl)
	inc hl
	ld a,(hl)
	inc hl
	ld h,(hl)
	ld l,a
	push bc
	ld bc,(kv_serial)
	or a
	sbc hl,bc
	pop bc
	jp nz,fail_bad_record
	ld h,d
	ld l,e
	ld d,b
	ld e,c
	ret

; in: HL = &starts[k], k below the stack depth.
; out: DE = start and BC = end of entry k. Destroys AF HL.
entry_bounds:
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	push de
	ld de,(kv_starts_end)
	or a
	sbc hl,de
	jr z,eb_last
	add hl,de
	ld c,(hl)
	inc hl
	ld b,(hl)
	jr eb_done
eb_last:
	ld bc,(kv_stack_top)
eb_done:
	pop de
	ret

; in: HL = expression in the used record, C = length.
; out: the expression with the bound variables replaced, built at the stack top and ending at
; kv_wr; refused if it would pass the stack symbol limit. Destroys AF BC DE HL.
substitute:
	ld de,(kv_stack_top)
	ld a,c
	or a
	jr z,sub_end
	ld b,c
sub_symbol:
	ld a,(hl)
	inc hl
	push hl
	push bc
	ld c,a
	ld h,TYPECODES>>8
	ld l,a
	ld a,(hl)
	inc a
	jr nz,sub_variable
	ld hl,(kv_sym_limit_ptr)
	or a
	sbc hl,de
	jp c,fail_oom
	jp z,fail_oom
	ld a,c
	ld (de),a
	inc de
	jr sub_next
sub_variable:
	ld a,c
	push de
	call subst_get
	ex de,hl
	or a
	sbc hl,de
	ld b,h
	ld c,l
	ex de,hl
	pop de
	push hl
	ld hl,(kv_sym_limit_ptr)
	or a
	sbc hl,de
	jp c,fail_oom
	or a
	sbc hl,bc
	jp c,fail_oom
	pop hl
	call copy_bc
sub_next:
	pop bc
	pop hl
	djnz sub_symbol
sub_end:
	ld (kv_wr),de
	ret

; in: B, C = two variables. out: returns if they are different and the theorem being checked
; lists them as a pair; the used record's page is mapped again.
pair_allowed:
	ld a,b
	cp c
	jp z,fail_disjoint
	call map_cur
	ld hl,(CUR_DESC+D_DV)
	ld de,(CUR_DESC+D_NDV)
	call pair_scan
	jr c,pa_found
	ld hl,(CUR_DESC+D_EXTRA)
	ld de,(CUR_DESC+D_NEXTRA)
	call pair_scan
	jp nc,fail_disjoint
pa_found:
	jp map_used

; in: HL = pairs, DE = count, B and C = the variables. out: C if some pair is {B, C}.
pair_scan:
ps_next:
	ld a,d
	or e
	jr z,ps_none
	dec de
	ld a,(hl)
	inc hl
	cp b
	jr nz,ps_other
	ld a,(hl)
	cp c
	jr z,ps_hit
	jr ps_skip
ps_other:
	cp c
	jr nz,ps_skip
	ld a,(hl)
	cp b
	jr z,ps_hit
ps_skip:
	inc hl
	jr ps_next
ps_hit:
	scf
	ret
ps_none:
	or a
	ret

; in: HL = number of an assertion earlier than the one being checked.
; out: the assertion applied to the top of the stack, or an error.
apply:
	ld ix,USED_DESC
	call parse_record
	call require_sound
	ld hl,(kv_nstack)
	ld a,(USED_DESC+D_NHYPS)
	ld e,a
	ld d,0
	or a
	sbc hl,de
	jp c,fail_underflow
	ld (kv_base),hl
	add hl,hl
	ld de,STARTS_BUF
	add hl,de
	ld (kv_entry0),hl
	ld hl,(kv_nstack)
	add hl,hl
	add hl,de
	ld (kv_starts_end),hl
	ld hl,(kv_serial)
	inc hl
	ld a,h
	or l
	jr nz,ap_serial
	call subst_clear
	ld hl,1
ap_serial:
	ld (kv_serial),hl

	ld a,(USED_DESC+D_NHYPS)
	or a
	jp z,ap_dv
	ld b,a
	ld hl,(USED_DESC+D_HYPS)
	ld de,(kv_entry0)
ap_float_pass:
	ld a,(hl)
	inc hl
	or a
	jr z,ap_float
	ld a,(hl)
	inc hl
	call add_hl_a
	jr ap_float_next
ap_float:
	ld a,(hl)
	inc hl
	ld (kv_var),a
	push hl
	push bc
	push de
	ex de,hl
	call entry_bounds
	ld a,(kv_var)
	ld l,a
	ld h,TYPECODES>>8
	ld a,(hl)
	ld h,d
	ld l,e
	cp (hl)
	jp nz,fail_typecode
	inc de
	push bc
	ld a,(kv_var)
	ld l,a
	ld h,0
	add hl,hl
	add hl,hl
	add hl,hl
	ld bc,SUBST
	add hl,bc
	pop bc
	ld (hl),e
	inc hl
	ld (hl),d
	inc hl
	ld (hl),c
	inc hl
	ld (hl),b
	inc hl
	ld de,(kv_serial)
	ld (hl),e
	inc hl
	ld (hl),d
	pop de
	pop bc
	pop hl
ap_float_next:
	inc de
	inc de
	djnz ap_float_pass

	ld a,(USED_DESC+D_NHYPS)
	ld b,a
	ld hl,(USED_DESC+D_HYPS)
	ld de,(kv_entry0)
ap_essential_pass:
	ld a,(hl)
	inc hl
	or a
	jr nz,ap_essential
	inc hl
	jr ap_essential_next
ap_essential:
	ld c,(hl)
	inc hl
	push hl
	push bc
	push de
	call substitute
	pop hl
	push hl
	call entry_bounds
	ld h,b
	ld l,c
	or a
	sbc hl,de
	push hl
	ld hl,(kv_wr)
	ld bc,(kv_stack_top)
	or a
	sbc hl,bc
	pop bc
	or a
	sbc hl,bc
	jp nz,fail_hyp_mismatch
	ld hl,(kv_stack_top)
	call mem_equal
	jp nz,fail_hyp_mismatch
	pop de
	pop bc
	pop hl
	ld a,c
	call add_hl_a
ap_essential_next:
	inc de
	inc de
	djnz ap_essential_pass

ap_dv:
	ld hl,(USED_DESC+D_DV)
	ld bc,(USED_DESC+D_NDV)
ap_pair:
	ld a,b
	or c
	jp z,ap_install
	dec bc
	push bc
	ld a,(hl)
	inc hl
	ld e,(hl)
	inc hl
	push hl
	push de
	call subst_get
	ld (kv_xs),hl
	ld (kv_xe),de
	pop de
	ld a,e
	call subst_get
	ld (kv_ys),hl
	ld (kv_ye),de
	ld hl,(kv_xs)
	ld (kv_xp),hl
ap_outer:
	ld hl,(kv_xp)
	ld de,(kv_xe)
	or a
	sbc hl,de
	jr nc,ap_pair_done
	add hl,de
	ld a,(hl)
	inc hl
	ld (kv_xp),hl
	ld (kv_a),a
	ld h,TYPECODES>>8
	ld l,a
	ld a,(hl)
	inc a
	jr z,ap_outer
	ld hl,(kv_ys)
	ld (kv_yp),hl
ap_inner:
	ld hl,(kv_yp)
	ld de,(kv_ye)
	or a
	sbc hl,de
	jr nc,ap_outer
	add hl,de
	ld a,(hl)
	inc hl
	ld (kv_yp),hl
	ld c,a
	ld h,TYPECODES>>8
	ld l,a
	ld a,(hl)
	inc a
	jr z,ap_inner
	ld a,(kv_a)
	ld b,a
	call pair_allowed
	jr ap_inner
ap_pair_done:
	pop hl
	pop bc
	jr ap_pair

ap_install:
	ld a,(USED_DESC+D_NCONC)
	ld c,a
	ld hl,(USED_DESC+D_CONC)
	call substitute
	ld hl,(kv_base)
	inc hl
	ld de,(kv_ent_limit)
	ex de,hl
	or a
	sbc hl,de
	jp c,fail_oom
	ld a,(USED_DESC+D_NHYPS)
	or a
	jr z,ap_bottom_top
	ld hl,(kv_entry0)
	ld e,(hl)
	inc hl
	ld d,(hl)
	jr ap_bottom
ap_bottom_top:
	ld de,(kv_stack_top)
ap_bottom:
	ld hl,(kv_entry0)
	ld (hl),e
	inc hl
	ld (hl),d
	ld hl,(kv_wr)
	ld bc,(kv_stack_top)
	or a
	sbc hl,bc
	ld b,h
	ld c,l
	ld hl,(kv_stack_top)
	call copy_bc
	ld (kv_stack_top),de
	ld hl,(kv_base)
	inc hl
	ld (kv_nstack),hl
	ret
