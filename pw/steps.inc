; The steps of the proof, one pass, inside the session of pw_walk: port 7 shows the theorem's page,
; HL walks the proof, and every table is on the spare page.

; in: A = an entry. Pushes it, or leaves with PW_ERR_STACK when 256 entries are there already.
pw_push:
	ld hl,(pv_nstack)
	ld c,a
	ld a,h
	or a
	jr nz,pw_stack_full
	ld de,PW_ENTRIES
	add hl,de
	ld (hl),c
	ld hl,(pv_nstack)
	inc hl
	ld (pv_nstack),hl
	ret
pw_stack_full:
	ld a,PW_ERR_STACK
	jp pw_leave

; in: pv_acc = the sizes of the new goal's subgoals added up. out: A = the new goal's number, its
; size, one more than pv_acc, stored. Leaves with PW_ERR_GOALS when 255 goals exist and with
; PW_ERR_TOTAL when the size passes 24 bits.
pw_mkgoal:
	ld a,(pv_goals)
	cp 255
	jr nc,pwk_goals
	ld hl,(pv_acc)
	ld de,1
	add hl,de
	ld a,(pv_acc+2)
	adc a,0
	jr c,pwk_total
	ld (pv_acc),hl
	ld (pv_acc+2),a
	ld a,(pv_goals)
	ld l,a
	ld h,0
	ld e,l
	ld d,h
	add hl,hl
	add hl,de
	ld de,PW_SIZES
	add hl,de
	ex de,hl
	ld hl,pv_acc
	ld bc,3
	ldir
	ld a,(pv_goals)
	ld hl,pv_goals
	inc (hl)
	ret
pwk_goals:
	ld a,PW_ERR_GOALS
	jp pw_leave
pwk_total:
	ld a,PW_ERR_TOTAL
	jp pw_leave

; in: A = a goal's number. Adds its size to pv_acc, or leaves with PW_ERR_TOTAL past 24 bits.
pw_addsize:
	ld l,a
	ld h,0
	ld e,l
	ld d,h
	add hl,hl
	add hl,de
	ld de,PW_SIZES
	add hl,de
	ld a,(pv_acc)
	add a,(hl)
	ld (pv_acc),a
	inc hl
	ld a,(pv_acc+1)
	adc a,(hl)
	ld (pv_acc+1),a
	inc hl
	ld a,(pv_acc+2)
	adc a,(hl)
	ld (pv_acc+2),a
	ret nc
	ld a,PW_ERR_TOTAL
	jp pw_leave

pw_proof:
	ld hl,(pv_proof)
	ld (pv_pp),hl
pwp_loop:
	ld hl,(pv_pp)
	ld de,(pv_pend)
	or a
	sbc hl,de
	jp nc,pwp_end
	add hl,de
	ld a,(hl)
	inc hl
	cp $FF
	jp z,pwp_save
	cp $F0
	jr c,pwp_small
	sub $F0
	ld b,a
	ld de,(pv_pend)
	push hl
	or a
	sbc hl,de
	pop hl
	jr c,pwp_wide
	ld a,PW_ERR_CUT
	jp pw_leave
pwp_wide:
	ld c,(hl)
	inc hl
	ld (pv_pp),hl
	ld hl,$F0
	add hl,bc
	ex de,hl
	jr pwp_number
pwp_small:
	ld (pv_pp),hl
	ld e,a
	ld d,0
pwp_number:
	ld hl,(pv_steps)
	inc hl
	ld (pv_steps),hl
	ld a,(pv_nhyps)
	ld c,a
	ld b,0
	ld h,d
	ld l,e
	or a
	sbc hl,bc
	jr c,pwp_hyp
	ld bc,(pv_nrefs)
	ld d,h
	ld e,l
	or a
	sbc hl,bc
	jr c,pwp_ref
	ld de,(pv_nsaved)
	push hl
	or a
	sbc hl,de
	pop hl
	jr c,pwp_reuse
	ld a,PW_ERR_BAD_REF
	jp pw_leave
pwp_reuse:
	ld de,PW_SAVED
	add hl,de
	ld a,(hl)
	call pw_push
	jp pwp_loop

pwp_hyp:
	ld hl,PW_HYPK
	add hl,de
	ld a,(hl)
	or a
	jr z,pwp_syntax
	xor a
	ld (pv_acc),a
	ld (pv_acc+1),a
	ld (pv_acc+2),a
	call pw_mkgoal
	call pw_push
	jp pwp_loop
pwp_syntax:
	ld a,$FF
	call pw_push
	jp pwp_loop

pwp_ref:
	ld hl,PW_MEMO
	add hl,de
	add hl,de
	ld b,(hl)
	inc hl
	ld c,(hl)
	ld hl,(pv_nstack)
	ld e,b
	ld d,0
	or a
	sbc hl,de
	jr nc,pwp_pop
	ld a,PW_ERR_UNDERFLOW
	jp pw_leave
pwp_pop:
	ld (pv_t1),hl
	ld a,c
	or a
	jr z,pwp_ref_syntax
	xor a
	ld (pv_acc),a
	ld (pv_acc+1),a
	ld (pv_acc+2),a
	ld (pv_t2),hl
pwp_child:
	ld hl,(pv_t2)
	ld de,(pv_nstack)
	or a
	sbc hl,de
	jr nc,pwp_made
	ld hl,(pv_t2)
	ld de,PW_ENTRIES
	add hl,de
	ld a,(hl)
	ld hl,(pv_t2)
	inc hl
	ld (pv_t2),hl
	cp $FF
	jp z,pwp_child
	ld hl,(pv_links)
	inc hl
	ld (pv_links),hl
	call pw_addsize
	jp pwp_child
pwp_made:
	call pw_mkgoal
	ld hl,(pv_t1)
	ld (pv_nstack),hl
	call pw_push
	jp pwp_loop
pwp_ref_syntax:
	ld hl,(pv_t1)
	ld (pv_nstack),hl
	ld a,$FF
	call pw_push
	jp pwp_loop

pwp_save:
	ld (pv_pp),hl
	ld hl,(pv_nstack)
	ld a,h
	or l
	jr nz,pwp_saving
	ld a,PW_ERR_SAVE_EMPTY
	jp pw_leave
pwp_saving:
	ld de,(pv_nsaved)
	ld a,d
	cp 2
	jr c,pwp_room
	ld a,PW_ERR_SAVED
	jp pw_leave
pwp_room:
	dec hl
	ld bc,PW_ENTRIES
	add hl,bc
	ld a,(hl)
	ld hl,PW_SAVED
	add hl,de
	ld (hl),a
	inc de
	ld (pv_nsaved),de
	jp pwp_loop

pwp_end:
	ld hl,(pv_nstack)
	ld a,h
	or l
	jr nz,pwe_some
	ld a,PW_ERR_NO_ENTRY
	jp pw_leave
pwe_some:
	dec hl
	ld a,h
	or l
	jr z,pwe_one
	ld a,PW_ERR_MANY_ENTRIES
	jp pw_leave
pwe_one:
	ld a,(PW_ENTRIES)
	cp $FF
	jr nz,pwe_goal
	ld a,PW_ERR_SYNTAX
	jp pw_leave
pwe_goal:
	ld l,a
	ld h,0
	ld e,l
	ld d,h
	add hl,hl
	add hl,de
	ld de,PW_SIZES
	add hl,de
	ld de,pv_total
	ld bc,3
	ldir
	xor a
	jp pw_leave
