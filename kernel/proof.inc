; Checking one assertion: its record and, for a theorem, its proof.

map_cur:
	ld a,(CUR_DESC+D_PAGE)
	out (7),a
	ret

map_used:
	ld a,(USED_DESC+D_PAGE)
	out (7),a
	ret

; in: HL = source, BC = length. out: pushed as a new stack entry; refused if the stack
; entries or symbols would pass their limits. Destroys AF BC DE HL.
stack_push:
	push hl
	push bc
	ld d,b
	ld e,c
	call work_charge
	ld hl,(kv_nstack)
	inc hl
	ld de,(kv_ent_limit)
	ex de,hl
	or a
	sbc hl,de
	jp c,fail_oom
	ld hl,(kv_stack_top)
	add hl,bc
	jp c,fail_oom
	ex de,hl
	ld hl,(kv_sym_limit_ptr)
	or a
	sbc hl,de
	jp c,fail_oom
	ld hl,(kv_nstack)
	add hl,hl
	ld de,STARTS_BUF
	add hl,de
	ld de,(kv_stack_top)
	ld (hl),e
	inc hl
	ld (hl),d
	pop bc
	pop hl
	call copy_bc
	ld (kv_stack_top),de
	ld hl,(kv_nstack)
	inc hl
	ld (kv_nstack),hl
	ret

; in: A = variable. out: its typecode and the variable pushed as an entry; Malformed if A is
; not a variable.
push_var:
	ld (kv_pair+1),a
	ld h,TYPECODES>>8
	ld l,a
	ld a,(hl)
	inc a
	jp z,fail_malformed
	dec a
	ld (kv_pair),a
	ld hl,kv_pair
	ld bc,2
	jp stack_push

; Marks the variables the current record has a float hypothesis for in FLOATS, which holds the
; marks of whichever record was checked last until then. Destroys AF BC DE HL.
build_floats:
	ld hl,FLOATS
	ld bc,256
	xor a
	call fill_bytes
	ld a,(CUR_DESC+D_NHYPS)
	or a
	ret z
	ld b,a
	ld hl,(CUR_DESC+D_HYPS)
bf_next:
	ld a,(hl)
	inc hl
	or a
	jr z,bf_float
	ld a,(hl)
	inc hl
	call add_hl_a
	djnz bf_next
	ret
bf_float:
	ld a,(hl)
	inc hl
	push hl
	ld h,FLOATS>>8
	ld l,a
	ld (hl),1
	pop hl
	djnz bf_next
	ret

; Refuses a current record whose reference list has a dummy entry for a variable it has a float
; hypothesis for, or two equal entries. Each entry examined for the first costs one unit of work
; and each comparison for the second one. A list of at most 511 entries costs at most 130,816
; units, which the count still holds when the call starts, so it is charged once, with the ON key
; read once per row of comparisons. A longer list is charged one unit before each step.
check_references:
	ld bc,(CUR_DESC+D_NREFS)
	ld a,b
	or c
	ret z
	xor a
	ld (kv_mode),a
	ld (kv_tmp8),a
	ld hl,511
	sbc hl,bc
	jr nc,cf_bulk
	ld a,1
	ld (kv_tmp8),a
cf_bulk:
	ld hl,(CUR_DESC+D_REFS)
cf_dummy:
	ld a,b
	or c
	jr z,cf_pairs
	dec bc
	ld a,(kv_tmp8)
	or a
	jr z,cf_free
	push hl
	call work_one
	pop hl
cf_free:
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld a,d
	cp $80
	jr nz,cf_dummy
	push hl
	push bc
	push de
	ld a,(kv_mode)
	or a
	call z,build_floats
	ld a,1
	ld (kv_mode),a
	pop de
	ld h,FLOATS>>8
	ld l,e
	ld a,(hl)
	pop bc
	pop hl
	or a
	jp nz,fail_bad_record
	jr cf_dummy
cf_pairs:
	ld bc,(CUR_DESC+D_NREFS)
	ld a,b
	or c
	ret z
	ld hl,511
	or a
	sbc hl,bc
	jp c,cf_slow
	push bc
	ld hl,362
	or a
	sbc hl,bc
	jr c,cf_big
	ld h,b
	ld l,c
	srl h
	rr l
	ex de,hl
	jr c,cf_odd
	dec bc
cf_odd:
	ld hl,0
cf_mul:
	srl b
	rr c
	jr nc,cf_skip
	add hl,de
cf_skip:
	sla e
	rl d
	ld a,b
	or c
	jr nz,cf_mul
	ld d,0
	jr cf_charge
cf_big:
	dec bc
	ld hl,0
	ld d,0
cf_sum:
	ld a,b
	or c
	jr z,cf_charge
	add hl,bc
	jr nc,cf_carried
	inc d
cf_carried:
	dec bc
	jr cf_sum
cf_charge:
	pop bc
	push bc
	add hl,bc
	jr nc,cf_included
	inc d
cf_included:
	ld (kv_tmp16),hl
	ld bc,(kv_tmp16)
	ld hl,(kv_work)
	or a
	sbc hl,bc
	ld (kv_work),hl
	ld a,(kv_work+2)
	sbc a,d
	ld (kv_work+2),a
	pop bc
	ld hl,(CUR_DESC+D_REFS)
cf_row:
	dec bc
	ld a,b
	or c
	ret z
	in a,(4)
	and 8
	jp z,fail_interrupted
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	push hl
	push bc
	ld a,e
cf_scan:
	cpi
	jr z,cf_low
	inc hl
	jp pe,cf_scan
cf_row_done:
	pop bc
	pop hl
	jr cf_row
cf_low:
	ld a,(hl)
	inc hl
	cp d
	jp z,fail_bad_record
	ld a,b
	or c
	ld a,e
	jr nz,cf_scan
	jr cf_row_done

; in: BC = entry count. The list in full, one unit of work before each comparison.
cf_slow:
	ld hl,(CUR_DESC+D_REFS)
cf_outer:
	ld a,b
	or c
	ret z
	dec bc
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld (kv_tmp16),de
	push hl
	push bc
cf_inner:
	ld a,b
	or c
	jr z,cf_next
	dec bc
	push hl
	call work_one
	pop hl
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld a,(kv_tmp16)
	cp e
	jr nz,cf_inner
	ld a,(kv_tmp16+1)
	cp d
	jp z,fail_bad_record
	jr cf_inner
cf_next:
	pop bc
	pop hl
	jr cf_outer

verify_body:
	xor a
	ld (kv_listed),a
	ld (kv_arg_id),hl
	call require_ready
	ld ix,CUR_DESC
	ld hl,(kv_arg_id)
	call parse_record
	ld a,1
	ld (kv_listed),a
	call require_sound
	ld a,(CUR_DESC+D_KIND)
	or a
	ret z
	ld hl,STACK_BUF
	ld (kv_stack_top),hl
	ld hl,SAVED_BUF
	ld (kv_saved_top),hl
	ld hl,(kv_saved_sym_limit)
	ld (kv_saved_free),hl
	ld hl,0
	ld (kv_nstack),hl
	ld (kv_nsaved),hl
	ld hl,(CUR_DESC+D_PROOF)
	ld (kv_proof_ptr),hl
	ld bc,(CUR_DESC+D_PLEN)
	add hl,bc
	ld (kv_proof_end),hl
	ld hl,WORK_LIMIT & $FFFF
	ld (kv_work),hl
	ld a,WORK_LIMIT >> 16
	ld (kv_work+2),a
	ld hl,(CUR_DESC+D_NDV)
	ld de,(CUR_DESC+D_NEXTRA)
	add hl,de
	inc hl
	ld (kv_charge),hl
	ld hl,0
	ld (kv_next),hl
	call check_references

vb_loop:
	in a,(4)
	and 8
	jp z,fail_interrupted
	ld hl,(kv_proof_ptr)
	ld de,(kv_proof_end)
	or a
	sbc hl,de
	jp nc,vb_done
	add hl,de
	ld a,(hl)
	inc hl
	cp $FF
	jp z,vb_save
	cp $F0
	jr nc,vb_wide
	ld (kv_proof_ptr),hl
	ld e,a
	ld d,0
	jr vb_dispatch
vb_wide:
	sub $F0
	ld b,a
	ld de,(kv_proof_end)
	push hl
	or a
	sbc hl,de
	pop hl
	jp nc,fail_malformed
	ld c,(hl)
	inc hl
	ld (kv_proof_ptr),hl
	ld hl,$F0
	add hl,bc
	ex de,hl
	jr vb_dispatch

vb_save:
	ld (kv_proof_ptr),hl
	ld hl,(kv_nstack)
	ld a,h
	or l
	jp z,fail_underflow
	dec hl
	add hl,hl
	ld de,STARTS_BUF
	add hl,de
	ld e,(hl)
	inc hl
	ld d,(hl)
	ld hl,(kv_stack_top)
	or a
	sbc hl,de
	ld b,h
	ld c,l
	push de
	push bc
	ld d,b
	ld e,c
	call work_charge
	ld hl,(kv_nsaved)
	inc hl
	ld de,(kv_saved_ent_limit)
	ex de,hl
	or a
	sbc hl,de
	jp c,fail_oom
	ld hl,(kv_saved_free)
	or a
	sbc hl,bc
	jp c,fail_oom
	ld (kv_saved_free),hl
	ld hl,(kv_nsaved)
	add hl,hl
	ld de,SAVED_STARTS_BUF
	add hl,de
	ld de,(kv_saved_top)
	ld (hl),e
	inc hl
	ld (hl),d
	pop bc
	pop hl
	call copy_bc
	ld (kv_saved_top),de
	ld hl,(kv_nsaved)
	inc hl
	ld (kv_nsaved),hl
	jp vb_loop

; in: DE = step number.
vb_dispatch:
	ld a,(CUR_DESC+D_NHYPS)
	ld c,a
	ld b,0
	ld h,d
	ld l,e
	or a
	sbc hl,bc
	jr c,vb_hyp
	ld bc,(CUR_DESC+D_NREFS)
	ld d,h
	ld e,l
	or a
	sbc hl,bc
	jp c,vb_ref
	jp vb_back

; in: E = hypothesis number below the count.
vb_hyp:
	ld hl,(CUR_DESC+D_HYPS)
	ld a,e
	or a
	jr z,vb_found
vb_skip:
	ld a,(hl)
	inc hl
	or a
	jr z,vb_skip_float
	ld c,(hl)
	ld b,0
	inc hl
	add hl,bc
	jr vb_skip_next
vb_skip_float:
	inc hl
vb_skip_next:
	dec e
	jr nz,vb_skip
vb_found:
	ld a,(hl)
	inc hl
	or a
	jr nz,vb_essential
	ld a,(hl)
	call push_var
	jp vb_loop
vb_essential:
	ld c,(hl)
	ld b,0
	inc hl
	call stack_push
	jp vb_loop

; in: DE = index into the reference list.
vb_ref:
	ld hl,(kv_next)
	or a
	sbc hl,de
	jr z,vb_ref_first
	jr nc,vb_ref_known
	jp fail_loose
vb_ref_first:
	ld hl,(kv_next)
	inc hl
	ld (kv_next),hl
vb_ref_known:
	ld hl,(CUR_DESC+D_REFS)
	add hl,de
	add hl,de
	ld e,(hl)
	inc hl
	ld d,(hl)
	bit 7,d
	jr nz,vb_dummy
	ld hl,(CUR_DESC+D_ID)
	ex de,hl
	or a
	sbc hl,de
	jp nc,fail_forward
	add hl,de
	call apply
	call map_cur
	jp vb_loop
vb_dummy:
	ld a,d
	and $7F
	jp nz,fail_malformed
	ld a,e
	call push_var
	jp vb_loop

; in: HL = saved step index plus the reference count taken off, as left by vb_dispatch.
vb_back:
	ld (kv_tmp16),hl
	ld de,(kv_nsaved)
	or a
	sbc hl,de
	jp nc,fail_back
	ld hl,(kv_tmp16)
	inc hl
	or a
	sbc hl,de
	jr c,vb_back_middle
	ld bc,(kv_saved_top)
	jr vb_back_end
vb_back_middle:
	ld hl,(kv_tmp16)
	inc hl
	add hl,hl
	ld de,SAVED_STARTS_BUF
	add hl,de
	ld c,(hl)
	inc hl
	ld b,(hl)
vb_back_end:
	ld hl,(kv_tmp16)
	add hl,hl
	ld de,SAVED_STARTS_BUF
	add hl,de
	ld e,(hl)
	inc hl
	ld d,(hl)
	ld h,b
	ld l,c
	or a
	sbc hl,de
	ld b,h
	ld c,l
	ex de,hl
	call stack_push
	jp vb_loop

vb_done:
	ld hl,(kv_next)
	ld de,(CUR_DESC+D_NREFS)
	or a
	sbc hl,de
	jp nz,fail_loose
	ld hl,(kv_nstack)
	dec hl
	ld a,h
	or l
	jp nz,fail_not_singleton
	ld hl,(kv_stack_top)
	ld de,STACK_BUF
	or a
	sbc hl,de
	ld a,(CUR_DESC+D_NCONC)
	ld c,a
	ld b,0
	or a
	sbc hl,bc
	jp nz,fail_wrong
	call map_cur
	ld hl,(CUR_DESC+D_CONC)
	ld de,STACK_BUF
	call mem_equal
	jp nz,fail_wrong
	ret
