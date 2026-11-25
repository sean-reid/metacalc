; Kind 5, the checking screen as Checking in crates/mcui/src/screens/checking.rs draws it: the stage's name
; and done/total in the title bar, "Checking" or "All checked", the bar, the label of the next assertion to
; check, "N failed", "ON pauses" and Stop or Done over F1. While the library's CRC is read, "Reading
; library" replaces the first line, the bar counts the chunks read of the chunks in the library, and there
; is no label and no failed count. It runs the check job (check.inc) between keys: idle does one unit of the
; job per call, so the shell looks for a key between every two.
; Frame: byte 1 the stage, byte 2 1 once the check has finished. Position only: the enter runs
; check_begin for the stage's end N and derives the phase in CKS_PHASE:
;   CKP_STEP    the table is counted: idle calls check_step
;   CKP_READ    the library has no table: idle calls check_crc_step, which reads the CRC and then opens it
;   CKP_DONE    nothing is left to check: idle touches nothing
;   CKP_LEAVE   ON stopped kernel_init: idle calls check_end and pops
; checking_enter  out: A = 0, or the number of the error notice's status when check_begin fails: ((class -
;                 CK_REFUSED) * 32) + detail, with class the job's status and detail its B masked to 5 bits,
;                 or 0 for CK_RANGE and CK_BAD_STATE. CK_REFUSED shows the kernel's status as 00 to 1F,
;                 CK_STORE the store's as 20 to 3F, CK_RANGE 40, CK_READER the reader's as 60 to 7F and
;                 CK_BAD_STATE 80. A finished frame whose table is counted stays finished, so a refresh
;                 after a power down shows "All checked" with no CRC run.
; checking_idle   in: HL = the frame. out: A and carry as scr_idle takes them. APP_ON set, which it clears,
;                 or CK_PAUSED, calls check_end and answers SCR_POP. CKP_DONE answers SCR_STAY with carry
;                 clear and touches nothing, APP_ON included. Stepping: one check_step; CK_MORE answers
;                 SCR_STAY with carry set, or SCR_REDRAW with carry set every CKS_EVERY steps since the last
;                 frame; CK_DONE calls check_end, sets the frame's byte 2 and answers SCR_REDRAW with carry
;                 clear. Reading: one check_crc_step; CK_MORE answers SCR_REDRAW with carry set, CK_READY
;                 starts the steps and answers the same, CK_REFUSED with B = 1, the CRC being wrong,
;                 replaces the screen with NOTICE_DAMAGED. Any other stopping status (CK_STORE, CK_READER,
;                 CK_REFUSED, CK_BAD_STATE) calls check_end and replaces the screen with the error notice. A
;                 replaced screen is the frame rewritten as the notice and entered with scr_refresh; the
;                 answer is SCR_REDRAW with carry clear, so CLEAR on it returns to Libraries.
; checking_key    F1 and CLEAR call check_end and pop; every other key stays.
; Interrupts and speed: at every return of checking_enter, checking_idle, checking_key and checking_draw
; both are as at the call. The kernel calls inside (check_begin, check_step, check_crc_step, check_end) go
; at 15 MHz with interrupts off, ck_fast saving the speed and ck_slow giving it back, the caller's state
; restored after each; far_lib_read, and far_ui_clear and lib_read through far_lib_label in the draw, turn
; interrupts off for their hops or copies and restore them; the store and the OS's lookups run at the
; caller's speed. checking_idle executes no DI or EI and writes no port 20h; checking_draw makes no kernel
; call, does not blit and touches neither elsewhere.
; Stack, from the shell's call of scr_idle to its return, interrupts off at the call: bytes written below
; the stack pointer there, scr_idle's return address counted. The shell's dispatch peaks at 10 before
; checking_idle, which starts with 4 on the stack; a figure above 10 is checking_idle's, its own use 4
; fewer. By call (stay, redraw, chunk, CRC run's end, finishing step, after the finish): stub OS 28, 28,
; 14, 30, 28, 10; reference OS 42, 42, 14, 44, 42, 10. The deepest, 30 and 44, includes the ON and
; CK_PAUSED stops, and on the stub OS a table lost, a reader failure and the damaged notice; the reference
; OS ran prop and two ON stops. An interrupt adds the handler's bytes: the greatest seen is 32 stub, 52 reference.
; Cost of scr_draw and ui_blit for a frame, cold, in T-states. Stub OS: 130,171 reading, 149,463 steps with
; a label, 125,129 finished, 128,488 damaged notice. Reference OS: 131,341, 155,820, 129,141.
; checking_draw   clears the canvas, stages checking_block, builds the strings in APP_TEXT and draws. done
;                 is N less the first count of the job and failed the third. The label is the library's
;                 label of the first assertion with value 0 from the job's cursor ck_i, found by
;                 store_next_zero, cut to its first 48 characters, which a title row never holds, then to
;                 the screen's width.

CKP_STEP	.equ 0
CKP_READ	.equ 1
CKP_DONE	.equ 2
CKP_LEAVE	.equ 3
CKS_EVERY	.equ 32
CKS_LABEL_MAX	.equ 48

CKS_PHASE	.equ APP_TEXT
CKS_SINCE	.equ APP_TEXT + 1
CKS_STAGE	.equ APP_TEXT + 2
CKS_CHUNKS	.equ APP_TEXT + 3
CKS_TOTAL	.equ APP_TEXT + 5
CKS_DONE	.equ APP_TEXT + 7
CKS_LEFT	.equ APP_TEXT + 9
CKS_FAILED	.equ APP_TEXT + 11
CKS_NAME	.equ APP_TEXT + 13
CKS_RIGHT	.equ CKS_NAME + 13
CKS_FAILTXT	.equ CKS_RIGHT + 12
CKS_RAW		.equ CKS_FAILTXT + 14
CKS_TEXT	.equ CKS_RAW + CKS_LABEL_MAX + 1
CKS_END		.equ CKS_TEXT + CKS_LABEL_MAX + 2
#if CKS_END > (APP_TEXT + APP_TEXT_SIZE)
 .error "the checking screen's variables are longer than APP_TEXT"
#endif

#ifdef LIBRARY
; In: HL = a frame.
checking_enter:
	push hl
	inc hl
	ld a, (hl)
	ld (CKS_STAGE), a
#ifdef STAGES_FROM_LIBRARY
	call far_lib_assertion_count
#else
	ld l, a
	ld h, 0
	add hl, hl
	ld de, stage_list + 1
	add hl, de
	ld e, (hl)
	inc hl
	ld d, (hl)
	ex de, hl
#endif
	ex de, hl
	ld a, (libPage)
	call check_begin
	pop hl
	or a
	jr z, kn_ready
	cp CK_NEED_CRC
	jr z, kn_read
	cp CK_PAUSED
	jr z, kn_paused
	jp checking_status
kn_ready:
	inc hl
	inc hl
	ld a, (hl)
	or a
	ld a, CKP_STEP
	jr z, kn_phase
	ld a, CKP_DONE
	jr kn_phase
kn_read:
	inc hl
	inc hl
	ld (hl), 0
	ld bc, 4
	ld de, CKS_RAW
	ld hl, 8
	xor a
	call far_lib_read
	or a
	jr nz, kn_reader
	ld hl, (CKS_RAW)
	ld a, (CKS_RAW + 2)
	ld e, a
	ld a, h
	or l
	jr nz, kn_low
	dec e
kn_low:
	dec hl
	ld a, h
	rlca
	rlca
	and 3
	ld d, a
	ld a, e
	add a, a
	add a, a
	or d
	ld l, a
	ld h, 0
	inc hl
	ld (CKS_CHUNKS), hl
	ld a, CKP_READ
	jr kn_phase
kn_reader:
	ld b, a
	push bc
	call check_end
	pop bc
	ld a, CK_READER
	jp checking_status
kn_paused:
	ld a, CKP_LEAVE
kn_phase:
	ld (CKS_PHASE), a
	xor a
	ld (CKS_SINCE), a
	ret

; In: A = the job's status, B = its detail. Out: A = the status the error notice shows.
checking_status:
	sub CK_REFUSED
	ld c, a
	ld a, b
	and 1Fh
	ld b, a
	ld a, c
	cp CK_RANGE - CK_REFUSED
	jr z, ks_bare
	cp CK_BAD_STATE - CK_REFUSED
	jr nz, ks_shift
ks_bare:
	ld b, 0
ks_shift:
	add a, a
	add a, a
	add a, a
	add a, a
	add a, a
	add a, b
	ret

; In: HL = a frame.
checking_idle:
	ld a, (CKS_PHASE)
	cp CKP_DONE
	jr nz, ki_work
	xor a
	ret
ki_work:
	push hl
	ld hl, APP_ON
	ld a, (hl)
	ld (hl), 0
	or a
	jr nz, ki_leave
	ld a, (CKS_PHASE)
	cp CKP_STEP
	jr z, ki_step
	cp CKP_READ
	jr z, ki_read
ki_leave:
	call check_end
	pop hl
	ld a, SCR_POP
	ret
ki_step:
	call check_step
	cp CK_MORE
	jr z, ki_more
	cp CK_DONE
	jr z, ki_finish
	cp CK_PAUSED
	jr z, ki_leave
	jr ki_fail
ki_more:
	pop hl
	ld hl, CKS_SINCE
	inc (hl)
	ld a, (hl)
	cp CKS_EVERY
	ld a, SCR_STAY
	jr nz, ki_pending
	ld (hl), 0
	ld a, SCR_REDRAW
ki_pending:
	scf
	ret
ki_finish:
	call check_end
	pop hl
	inc hl
	inc hl
	ld (hl), 1
	ld a, CKP_DONE
	ld (CKS_PHASE), a
	ld a, SCR_REDRAW
	or a
	ret
ki_read:
	call check_crc_step
	cp CK_MORE
	jr z, ki_chunk
	or a
	jr z, ki_ready
	cp CK_PAUSED
	jr z, ki_leave
	cp CK_REFUSED
	jr nz, ki_fail
	ld a, b
	cp 1
	ld a, CK_REFUSED
	jr nz, ki_fail
	call check_end
	pop hl
	ld b, NOTICE_DAMAGED
	ld c, 0
	jr ki_replace
ki_ready:
	xor a
	ld (CKS_SINCE), a
	ld a, CKP_STEP
	ld (CKS_PHASE), a
ki_chunk:
	pop hl
	ld a, SCR_REDRAW
	scf
	ret
ki_fail:
	call checking_status
	push af
	call check_end
	pop af
	ld c, a
	pop hl
	ld b, NOTICE_ERROR
ki_replace:
	ld a, KIND_NOTICE
	call screen_make
	call scr_refresh
	ld a, SCR_REDRAW
	or a
	ret

; In: A = the key, HL = a frame.
checking_key:
	cp KEY_F1
	jr z, kk_stop
	cp KEY_CLEAR
	ld a, SCR_STAY
	ret nz
kk_stop:
	call check_end
	ld a, SCR_POP
	ret

; In: HL = a frame.
checking_draw:
	inc hl
	ld a, (hl)
	ld (CKS_STAGE), a
	call far_ui_clear
	ld hl, checking_block
	ld bc, CHECKING_BLOCK_SIZE
	call app_stage_block
	ld hl, (ck_n)
	ld (CKS_TOTAL), hl
	ld de, 0
	ld bc, 0
	ld a, (CKS_PHASE)
	cp CKP_READ
	jr nz, kd_counted
	ld hl, 1
	jr kd_store
kd_counted:
	ld hl, (ck_counts)
	cp CKP_LEAVE
	jr nz, kd_real
	ld hl, (ck_n)
	jr kd_store
kd_real:
	ld de, (ck_n)
	ex de, hl
	or a
	sbc hl, de
	ex de, hl
	ld bc, (ck_counts + 4)
kd_store:
	ld (CKS_LEFT), hl
	ld (CKS_DONE), de
	ld (CKS_FAILED), bc
	ld a, (CKS_STAGE)
	call stage_strings
	ld de, CKS_NAME
	call app_strcpy
	ld hl, (CKS_DONE)
	ld de, CKS_RIGHT
	call app_dec16
	ld a, UI_FONT_CH_2F
	ld (de), a
	inc de
	ld hl, (CKS_TOTAL)
	call app_dec16
	xor a
	ld (de), a
	ld hl, CKS_NAME
	ld de, CKS_RIGHT
	call far_ui_title
	ld a, (CKS_PHASE)
	cp CKP_READ
	ld hl, STG_CK_READING
	jr z, kd_row1
	ld hl, (CKS_LEFT)
	ld a, h
	or l
	ld hl, STG_CK_ALL
	jr z, kd_row1
	ld hl, STG_CK_CHECKING
kd_row1:
	ld b, 1
	call far_ui_text_centered
	ld a, (CKS_PHASE)
	cp CKP_READ
	jr z, kd_bar_read
	ld hl, (CKS_DONE)
	ld de, (CKS_TOTAL)
	jr kd_bar
kd_bar_read:
	ld a, (ck_chunks)
	ld l, a
	ld h, 0
	ld de, (CKS_CHUNKS)
kd_bar:
	ld a, 2
	call far_ui_bar
	ld a, (CKS_PHASE)
	cp CKP_STEP
	jr nz, kd_failed
	ld hl, (CKS_LEFT)
	ld a, h
	or l
	call nz, checking_label
kd_failed:
	ld a, (CKS_PHASE)
	cp CKP_READ
	jr z, kd_pauses
	ld hl, (CKS_FAILED)
	ld de, CKS_FAILTXT
	call app_dec16
	ld hl, ck_failed
	call app_strcpy
	ld hl, CKS_FAILTXT
	ld b, 5
	call far_ui_text_centered
kd_pauses:
	ld hl, (CKS_LEFT)
	ld a, h
	or l
	ld hl, STG_CK_KEYS_DONE
	jr z, kd_keys
	ld hl, STG_CK_PAUSES
	ld b, 6
	call far_ui_text_centered
	ld hl, STG_CK_KEYS_STOP
kd_keys:
	jp far_ui_soft_keys

; Draws the label of the first assertion with value 0 from ck_i on, as the third line; nothing when
; the table or the library refuses.
checking_label:
	ld hl, (ck_i)
	ld de, (ck_n)
	call store_next_zero
	or a
	ret nz
	ld de, FM_SCRATCH
	call far_lib_label
	or a
	ret nz
	ld a, b
	cp CKS_LABEL_MAX + 1
	jr c, kl_count
	ld a, CKS_LABEL_MAX
kl_count:
	ld b, a
	ld hl, FM_SCRATCH
	ld de, CKS_RAW
kl_glyph:
	ld a, b
	or a
	jr z, kl_cut
	push bc
	ld a, (hl)
	inc hl
	push hl
	sub 20h
	cp 95
	jr c, kl_known
	ld a, 3Fh - 20h
kl_known:
	ld c, a
	ld b, 0
	ld hl, checking_glyphs
	add hl, bc
	ld a, (hl)
	ld (de), a
	inc de
	pop hl
	pop bc
	dec b
	jr kl_glyph
kl_cut:
	xor a
	ld (de), a
	ld hl, CKS_RAW
	ld de, CKS_TEXT
	ld b, CKS_LABEL_MAX + 2
	ld c, 94
	call far_ui_truncate
	ld hl, CKS_TEXT
	ld b, 3
	jp far_ui_text_centered

; The glyph code of each printable ASCII character from the space to the tilde.
checking_glyphs:
	.db UI_FONT_CH_20, UI_FONT_CH_21, UI_FONT_CH_22, UI_FONT_CH_23, UI_FONT_CH_24, UI_FONT_CH_25
	.db UI_FONT_CH_26, UI_FONT_CH_27, UI_FONT_CH_28, UI_FONT_CH_29, UI_FONT_CH_2A, UI_FONT_CH_2B
	.db UI_FONT_CH_2C, UI_FONT_CH_2D, UI_FONT_CH_2E, UI_FONT_CH_2F, UI_FONT_CH_30, UI_FONT_CH_31
	.db UI_FONT_CH_32, UI_FONT_CH_33, UI_FONT_CH_34, UI_FONT_CH_35, UI_FONT_CH_36, UI_FONT_CH_37
	.db UI_FONT_CH_38, UI_FONT_CH_39, UI_FONT_CH_3A, UI_FONT_CH_3B, UI_FONT_CH_3C, UI_FONT_CH_3D
	.db UI_FONT_CH_3E, UI_FONT_CH_3F, UI_FONT_CH_40, UI_FONT_CH_41, UI_FONT_CH_42, UI_FONT_CH_43
	.db UI_FONT_CH_44, UI_FONT_CH_45, UI_FONT_CH_46, UI_FONT_CH_47, UI_FONT_CH_48, UI_FONT_CH_49
	.db UI_FONT_CH_4A, UI_FONT_CH_4B, UI_FONT_CH_4C, UI_FONT_CH_4D, UI_FONT_CH_4E, UI_FONT_CH_4F
	.db UI_FONT_CH_50, UI_FONT_CH_51, UI_FONT_CH_52, UI_FONT_CH_53, UI_FONT_CH_54, UI_FONT_CH_55
	.db UI_FONT_CH_56, UI_FONT_CH_57, UI_FONT_CH_58, UI_FONT_CH_59, UI_FONT_CH_5A, UI_FONT_CH_5B
	.db UI_FONT_CH_5C, UI_FONT_CH_5D, UI_FONT_CH_5E, UI_FONT_CH_5F, UI_FONT_CH_60, UI_FONT_CH_61
	.db UI_FONT_CH_62, UI_FONT_CH_63, UI_FONT_CH_64, UI_FONT_CH_65, UI_FONT_CH_66, UI_FONT_CH_67
	.db UI_FONT_CH_68, UI_FONT_CH_69, UI_FONT_CH_6A, UI_FONT_CH_6B, UI_FONT_CH_6C, UI_FONT_CH_6D
	.db UI_FONT_CH_6E, UI_FONT_CH_6F, UI_FONT_CH_70, UI_FONT_CH_71, UI_FONT_CH_72, UI_FONT_CH_73
	.db UI_FONT_CH_74, UI_FONT_CH_75, UI_FONT_CH_76, UI_FONT_CH_77, UI_FONT_CH_78, UI_FONT_CH_79
	.db UI_FONT_CH_7A, UI_FONT_CH_7B, UI_FONT_CH_7C, UI_FONT_CH_7D, UI_FONT_CH_7E
#else
checking_enter:
checking_draw:
checking_idle:
	xor a
	ret
checking_key:
	xor a
	ret
#endif
