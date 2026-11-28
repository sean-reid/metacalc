; pw_shape: the entry, the session and the tables the proof's steps need.

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
	ld (lr_bufp),de
	ld a,b
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
	ld a,i
	jp pe,pws_on
	ld a,i
	jp pe,pws_on
	ld b,0
	jr pws_off
pws_on:
	ld b,1
pws_off:
	di
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
	ld (pv_sp),sp
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
	ld hl,(lp_nrefs)
	ld (pv_nrefs),hl
	ld a,(lp_page)
	ld (pv_rpage),a
	PW_ADDR(lr_dhyps, pv_hyps)
	PW_ADDR(lp_refs, pv_refs)
	PW_ADDR(lp_proof, pv_proof)
	ld hl,(lp_plen)
	ld de,(pv_proof)
	add hl,de
	ld (pv_pend),hl
pws_window_end:
	ld sp,PW_STACK_TOP
	ld a,(pv_rpage)
	out (7),a
	ld hl,pv_steps
	ld de,pv_steps+1
	ld bc,PW_CLEAR_SIZE-1
	xor a
	ld (hl),a
	ldir
	call pw_tables
	jp pw_proof

; Reads the theorem's hypotheses into PW_HYPK and each reference's record head into PW_MEMO.
pw_tables:
	ld a,(pv_nhyps)
	or a
	jr z,pwt_refs
	ld (pv_cnt),a
	ld hl,(pv_hyps)
	ld de,PW_HYPK
pwt_hyp:
	ld a,(hl)
	inc hl
	or a
	jr nz,pwt_essential
	inc hl
	xor a
	jr pwt_store
pwt_essential:
	ld c,(hl)
	inc hl
	ld b,0
	ld a,c
	or a
	jr z,pwt_syntax
	ld a,(pv_tc)
	cp (hl)
	ld a,1
	jr z,pwt_skip
pwt_syntax:
	xor a
pwt_skip:
	add hl,bc
pwt_store:
	ld (de),a
	inc de
	ld a,(pv_cnt)
	dec a
	ld (pv_cnt),a
	jr nz,pwt_hyp
pwt_refs:
	ld hl,(pv_refs)
	ld (pv_rp),hl
	ld hl,PW_MEMO
	ld (pv_mp),hl
	ld hl,(pv_nrefs)
	ld (pv_ri),hl
pwt_ref:
	ld hl,(pv_ri)
	ld a,h
	or l
	ret z
	dec hl
	ld (pv_ri),hl
	ld hl,(pv_rp)
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld (pv_rp),hl
	bit 7,d
	jr nz,pwt_dummy
	ld hl,(pv_id)
	ex de,hl
	or a
	sbc hl,de
	jr c,pwt_earlier
	ld a,PW_ERR_FORWARD
	jp pw_leave
pwt_earlier:
	add hl,de
	call pw_head
	jp nz,pw_leave
	ld hl,(pv_mp)
	ld (hl),b
	inc hl
	ld (hl),c
	inc hl
	ld (pv_mp),hl
	ld a,(pv_rpage)
	out (7),a
	jr pwt_ref
pwt_dummy:
	ld hl,(pv_mp)
	xor a
	ld (hl),a
	inc hl
	ld (hl),a
	inc hl
	ld (pv_mp),hl
	jr pwt_ref

; in: A = status. Puts the caller's ports, stack and interrupt state back, writes the result block
; when the status is 0, and returns to the caller of pw_shape with the zero flag set exactly when
; the status is 0.
pw_leave:
	ld (pv_status),a
	ld a,(pv_p7)
	out (7),a
	ld a,(pv_status)
	or a
	jr nz,pwl_back
	ld hl,PW_RESULT
	ld de,(lr_bufp)
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
