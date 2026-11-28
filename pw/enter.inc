; The three entries, the session that maps the spare page, and the exit every call leaves by.

; Stores at `dst` the bank B address of the 24 bit library offset at `src`: its low 14 bits with
; bit 15 set. Destroys A.
#macro PW_ADDR(src, dst)
	ld a,(src+1)
	and $3F
	or $80
	ld (dst+1),a
	ld a,(src)
	ld (dst),a
#endmacro

pw_shape:
	ld (lr_scratch),bc
	ld (lr_bufp),de
	ld (lr_k),a
	ld (lr_cn),hl
	call lr_need
	ret nz
	ld de,(lr_bufp)
	ld bc,PW_RESULT_SIZE
	call lr_chkbuf
	ret nz
	ld hl,(lr_cn)
	call lib_proof
	ret nz
	ld a,(lr_dclen)
	or a
	jp z,lr_bad
	ld bc,1
	ld de,lr_hdr
	ld hl,(lr_dconc)
	ld a,(lr_dconc+2)
	call lr_rd
	ret nz
	ld a,(lr_hdr)
	ld hl,lr_k
	cp (hl)
	jr z,pws_provable
	ld a,PW_ERR_SYNTAX
	or a
	ret
pws_provable:
	ld hl,(lp_nrefs)
	ld de,PW_MAX_REFS + 1
	or a
	sbc hl,de
	jr c,pws_refs_fit
	ld a,PW_ERR_REFS
	or a
	ret
pws_refs_fit:
	call pw_iff
	ld e,0
	jp pw_session

pw_more:
	ld (lr_scratch),bc
	ld (lr_bufp),de
	ld bc,PW_RESULT_SIZE
	call lr_chkbuf
	ret nz
	ld e,PW_MARK_RUN
	call pw_iff
	jp pw_peek

pw_find:
	ld (lr_tr),hl
	ld (lr_tr+2),a
	ld (lr_bufp),de
	ld bc,PW_FIND_SIZE
	call lr_chkbuf
	ret nz
	ld e,PW_MARK_DONE
	call pw_iff
	jp pw_peek

; out: B = 1 when interrupts were on, else 0; interrupts off. Destroys A.
pw_iff:
	ld a,i
	jp pe,pwi_on
	ld a,i
	jp pe,pwi_on
	ld b,0
	di
	ret
pwi_on:
	ld b,1
	di
	ret

; in: B = the interrupt flag, E = the mark the caller needs, interrupts off. Maps the spare page,
; compares the mark and the sum of the persistent variables with what is stored, writes nothing,
; puts port 5 back, and either enters the session for the entry that asked, or returns its status
; to that entry's caller. No instruction between the two port 5 writes touches the stack.
pw_peek:
	in a,(5)
	ld c,a
	ld a,KERNEL_RAM_PAGE
pwk_window:
	out (5),a
	ld a,(pv_mark)
	cp e
	jr nz,pwk_bad
	ld hl,PW_PERSIST_END - 1
	ld d,0
	xor a
pwk_sum:
	add a,(hl)
	jr nc,pwk_next
	inc d
pwk_next:
	dec l
	jr nz,pwk_sum
	ld hl,(pv_sum)
	cp l
	jr nz,pwk_bad
	ld a,d
	cp h
	jr nz,pwk_bad
	ld d,0
	jr pwk_back
pwk_bad:
	ld d,1
pwk_back:
	ld a,c
	out (5),a
pwk_window_end:
	ld a,d
	or a
	jr nz,pwk_none
	ld a,e
	cp PW_MARK_RUN
	ld e,1
	jp z,pw_session
	ld e,2
	jp pw_session
pwk_none:
	ld a,e
	cp PW_MARK_RUN
	ld a,PW_ERR_NO_WALK
	jr z,pwk_say
	ld a,PW_ERR_NO_TABLE
pwk_say:
	ld c,a
	ld a,b
	or a
	ld a,c
	jr z,pwk_off
	or a
	ei
	ret
pwk_off:
	or a
	ret

; in: B = the interrupt flag, E = 0 for a start, 1 for a continue, 2 for a find, interrupts off.
; The caller's ports and stack pointer are kept in the spare page and the walk goes on the private
; stack. No instruction between the two stack changes touches the stack.
pw_session:
	in a,(7)
	ld d,a
	in a,(5)
	ld c,a
	ld a,KERNEL_RAM_PAGE
pws_window:
	out (5),a
	ld a,b
	ld (pv_iff),a
	ld a,c
	ld (pv_p5),a
	ld a,d
	ld (pv_p7),a
	ld a,e
	ld (pv_mode),a
	ld (pv_sp),sp
pws_window_end:
	ld sp,PW_STACK_TOP
	ld hl,(lr_scratch)
	ld (pv_left),hl
	ld a,h
	or l
	jr z,pws_uncut
	ld a,1
pws_uncut:
	ld (pv_cut),a
	ld hl,(lr_bufp)
	ld (pv_blk),hl
	ld a,(pv_mode)
	or a
	jr z,pw_start
	dec a
	jp z,pw_resume
	jp pw_find_run

pw_start:
	xor a
	ld (pv_mark),a
	ld a,(lr_k)
	ld (pv_tc),a
	ld a,(lr_page)
	ld (pv_lpage),a
	ld hl,(lr_dir)
	ld (pv_dir),hl
	ld a,(lr_dir+2)
	ld (pv_dir+2),a
	ld hl,(lr_length)
	ld (pv_len),hl
	ld a,(lr_length+2)
	ld (pv_len+2),a
	ld hl,(lr_dn)
	ld (pv_id),hl
	ld a,(lr_dnhyps)
	ld (pv_nhyps),a
	ld (pv_hrem),a
	ld hl,(lp_nrefs)
	ld (pv_nrefs),hl
	ld (pv_ri),hl
	ld a,(lp_page)
	ld (pv_rpage),a
	PW_ADDR(lr_dhyps, pv_hyps)
	PW_ADDR(lp_refs, pv_refs)
	PW_ADDR(lp_proof, pv_proof)
	ld hl,(pv_refs)
	ld (pv_rp),hl
	ld hl,(lp_plen)
	ld de,(pv_proof)
	ld (pv_pp),de
	add hl,de
	ld (pv_pend),hl
	ld hl,PW_MEMO
	ld (pv_mp),hl
	ld hl,PW_HYPK
	ld (pv_hk),hl
	xor a
	ld (pv_stage),a
	ld (pv_rank),a
	ld hl,pv_steps
	ld de,pv_steps+1
	ld bc,PW_CLEAR_SIZE-1
	ld (hl),a
	ldir
	jr pw_run

pw_resume:
	xor a
	ld (pv_mark),a
pw_run:
	ld a,(pv_stage)
	or a
	jp z,pw_hyps
	dec a
	jp z,pw_refs
	jp pw_proof

; The top of every unit of work: returns when the slice has a unit left, else ends the call with
; PW_ST_MORE. Everything the walk needs to go on is in the variables and tables here.
pw_unit:
	ld a,(pv_cut)
	or a
	ret z
	ld hl,(pv_left)
	ld a,h
	or l
	jr z,pwu_more
	dec hl
	ld (pv_left),hl
	ret
pwu_more:
	ld a,PW_ST_MORE
	jp pw_leave

; out: HL = the sum of the persistent variables, as pw_peek adds them. Destroys A, D.
pw_sum:
	ld hl,PW_PERSIST_END - 1
	ld d,0
	xor a
pwx_loop:
	add a,(hl)
	jr nc,pwx_next
	inc d
pwx_next:
	dec l
	jr nz,pwx_loop
	ld l,a
	ld h,d
	ret

; in: A = a status. Seals a walk that goes on or is done, puts the caller's ports, stack and
; interrupt state back, writes the result block when a start or a continue ends with 0, and returns
; to the caller of the entry with the zero flag set exactly when the status is 0.
pw_leave:
	ld (pv_status),a
	ld a,(pv_mode)
	cp 2
	jr z,pwl_marked
	ld a,(pv_status)
	ld c,PW_MARK_RUN
	cp PW_ST_MORE
	jr z,pwl_seal
	or a
	jr nz,pwl_marked
	ld c,PW_MARK_DONE
pwl_seal:
	call pw_sum
	ld (pv_sum),hl
	ld a,c
	ld (pv_mark),a
pwl_marked:
	ld a,(pv_p7)
	out (7),a
	ld a,(pv_status)
	or a
	jr nz,pwl_back
	ld a,(pv_mode)
	cp 2
	jr z,pwl_back
	ld hl,PW_RESULT
	ld de,(pv_blk)
	ld bc,PW_RESULT_SIZE
	ldir
pwl_back:
	ld a,(pv_iff)
	ld d,a
	ld a,(pv_p5)
	ld e,a
	ld a,(pv_status)
	ld b,a
	ld sp,(pv_sp)
pwl_window:
	ld a,e
	out (5),a
pwl_window_end:
	ld a,d
	or a
	ld a,b
	jr z,pwl_off
	or a
	ei
	ret
pwl_off:
	or a
	ret
