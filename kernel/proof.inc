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

verify_body:
	ld hl,(kv_ready)
	ld de,READY_MAGIC
	or a
	sbc hl,de
	jp nz,fail_malformed
	ld ix,CUR_DESC
	ld hl,(kv_arg_id)
	call parse_record
	call require_sound
	ld a,(CUR_DESC+D_KIND)
	or a
	ret z
	ld hl,STACK_BUF
	ld (kv_stack_top),hl
	ld hl,SAVED_BUF
	ld (kv_saved_top),hl
	ld hl,0
	ld (kv_nstack),hl
	ld (kv_nsaved),hl
	ld hl,(CUR_DESC+D_PROOF)
	ld (kv_proof_ptr),hl
	ld bc,(CUR_DESC+D_PLEN)
	add hl,bc
	ld (kv_proof_end),hl

vb_loop:
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
	ld hl,(kv_nsaved)
	inc hl
	ld de,(kv_saved_ent_limit)
	ex de,hl
	or a
	sbc hl,de
	jp c,fail_oom
	ld hl,(kv_saved_top)
	add hl,bc
	jp c,fail_oom
	ex de,hl
	ld hl,(kv_saved_sym_ptr)
	or a
	sbc hl,de
	jp c,fail_oom
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
