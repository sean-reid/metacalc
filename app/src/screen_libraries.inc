; Kind 1, the Libraries screen as Libraries in crates/mcui/src/screens/libraries.rs draws it: the
; title bar "Libraries" and "metacalc", one row for each stage of the table (screen_counts.inc) and
; then "My proofs" with its count, 0, in the rows 1 to 6 from the first one shown, and the keys
; Open, Run and Info. The model's rows are the stages of every installed library, and this app has
; one. Without LIBRARY there are no stage rows and STAGE_COUNT is 0.
;
; Frame: byte 1 the cursor, byte 2 the first row shown. Position only: libs_enter derives the counts
; (libs_counts) and, for each stage, its row's two strings in LIBS_SLOTS: the progress text, and
; the name cut to the room that text leaves. libs_draw only draws them. Building them in the enter,
; which runs once for each opening, keeps a frame from paying for the cut and the decimal digits.
;
; A stage row's right text is the model's progress text: nothing when the stage has no assertion,
; the failed mark when any assertion below its end failed, else the checked mark when all are
; checked, the unchecked mark when none is, else "checked/total", checked being n1 + n3 of the
; store. The model's "damaged" is not reachable here: no CRC run happens on this screen. The
; counts come from a table with this library's CRC and count; with none, an archived one or one the
; count pass refuses, every mark shows as unchecked until a check runs.
;
; libs_enter   A = appStatus when the reader or fm refused the library, so the shell shows the
;              error notice, else the status of libs_counts (0 for any table), then the strings are built.
; libs_key     UP and DOWN move the cursor, stopping at the ends, and the first row shown moves as
;              little as it must to keep the cursor on screen; a key that moves nothing is SCR_STAY.
;              ENTER and F1 push the notice of the list (NOTICE_LIST, titled by the stage) or, on
;              My proofs, NOTICE_PROOFS; F2 pushes NOTICE_VERIFY or, on My proofs, NOTICE_PICK; F3
;              pushes KIND_STAGE for a stage of a library with more than one stage, else About;
;              CLEAR pops, which leaves the app. Every other key stays.
; libs_draw    clears the canvas, stages libs_block and draws the rows.

LIBS_VISIBLE	.equ 6
LIBS_NAME	.equ 13
LIBS_RIGHT_SIZE	.equ 12
LIBS_SLOT_SIZE	.equ LIBS_NAME + LIBS_RIGHT_SIZE

LIBS_IDX	.equ APP_TEXT
LIBS_ROW	.equ APP_TEXT + 1
LIBS_CURSOR	.equ APP_TEXT + 2
LIBS_TOP	.equ APP_TEXT + 3
LIBS_TOTAL	.equ APP_TEXT + 4
LIBS_CHECKED	.equ APP_TEXT + 6
LIBS_FAILED	.equ APP_TEXT + 8
LIBS_RAW	.equ APP_TEXT + 10
LIBS_SLOTS	.equ LIBS_RAW + LIBS_NAME
#if (LIBS_SLOTS + (STAGE_COUNT * LIBS_SLOT_SIZE)) > (APP_TEXT + APP_TEXT_SIZE)
 .error "the Libraries screen's variables are longer than APP_TEXT"
#endif

libs_enter:
#ifdef LIBRARY
	ld a, (appStatus)
	or a
	ret nz
	call libs_counts
	or a
	ret nz
#if STAGE_COUNT > 0
	xor a
	ld (LIBS_IDX), a
libs_cut:
	ld a, (LIBS_IDX)
	call libs_slot
	ld bc, LIBS_NAME
	add hl, bc
	ld a, (LIBS_IDX)
	call libs_progress
	ld a, (LIBS_IDX)
	call stage_strings
	ld de, LIBS_RAW
	call app_strcpy
	ld a, (LIBS_IDX)
	call libs_slot
	push hl
	ld bc, LIBS_NAME
	add hl, bc
	ld a, (hl)
	push af
	call far_ui_text_width
	pop af
	ld b, 94
	or a
	jr z, libs_room
	ld b, 91
libs_room:
	ld a, b
	sub l
	jr nc, libs_fit
	xor a
libs_fit:
	ld c, a
	pop de
	ld hl, LIBS_RAW
	ld b, LIBS_NAME
	call far_ui_truncate
	ld hl, LIBS_IDX
	inc (hl)
	ld a, (hl)
	cp STAGE_COUNT
	jr c, libs_cut
#endif
	xor a
	ret
#else
	xor a
	ret
#endif

libs_draw:
	inc hl
	ld a, (hl)
	ld (LIBS_CURSOR), a
	inc hl
	ld a, (hl)
	ld (LIBS_TOP), a
	call far_ui_clear
	ld hl, libs_block
	ld bc, LIBS_BLOCK_SIZE
	call app_stage_block
	ld hl, STG_LIBS_TITLE
	ld de, STG_LIBS_RIGHT
	call far_ui_title
	xor a
	ld (LIBS_ROW), a
libs_rows:
	ld a, (LIBS_ROW)
	ld hl, LIBS_TOP
	add a, (hl)
	ld (LIBS_IDX), a
	cp STAGE_COUNT
#if STAGE_COUNT > 0
	jr c, libs_stage_row
#endif
	jr z, libs_proofs_row
	jr libs_keys
#if STAGE_COUNT > 0
libs_stage_row:
	ld a, (LIBS_IDX)
	call libs_slot
	push hl
	ld bc, LIBS_NAME
	add hl, bc
	ex de, hl
	pop hl
	call libs_list_row
	ld hl, LIBS_ROW
	inc (hl)
	ld a, (hl)
	cp LIBS_VISIBLE
	jr c, libs_rows
	jr libs_keys
#endif
libs_proofs_row:
	ld hl, STG_LIBS_PROOFS
	ld de, STG_LIBS_ZERO
	call libs_list_row
libs_keys:
	ld hl, STG_LIBS_KEYS
	jp far_ui_soft_keys

; In: HL = the left text, DE = the right text. Draws the row LIBS_ROW + 1, selected when LIBS_IDX is
; the cursor.
libs_list_row:
	ld a, (LIBS_IDX)
	ld bc, (LIBS_CURSOR)
	cp c
	ld c, 0
	jr nz, libs_unselected
	inc c
libs_unselected:
	ld a, (LIBS_ROW)
	inc a
	ld b, a
	jp far_ui_list_row

#if STAGE_COUNT > 0
; In: A = a stage. Out: HL = its slot in LIBS_SLOTS, the name cut to fit and then the progress text.
; Destroys AF BC DE.
libs_slot:
	ld c, a
	ld b, 0
	ld l, a
	ld h, 0
	add hl, hl
	add hl, hl
	add hl, hl
	ld d, h
	ld e, l
	add hl, hl
	add hl, de
	add hl, bc
	ld de, LIBS_SLOTS
	add hl, de
	ret

; In: A = a stage, HL = where to write its progress text and the 0 after it.
libs_progress:
	push hl
	call libs_figures
	ld (LIBS_TOTAL), hl
	ld (LIBS_CHECKED), de
	ld (LIBS_FAILED), bc
	pop hl
	ld bc, (LIBS_TOTAL)
	ld a, b
	or c
	jr z, lp_end
	ld bc, (LIBS_FAILED)
	ld a, b
	or c
	jr z, lp_clean
	ld (hl), UI_FONT_CH_2717
	inc hl
	jr lp_end
lp_clean:
	ld de, (LIBS_CHECKED)
	ld bc, (LIBS_TOTAL)
	ld a, d
	or e
	jr z, lp_none
	ld a, d
	cp b
	jr nz, lp_part
	ld a, e
	cp c
	jr nz, lp_part
	ld (hl), UI_FONT_CH_2713
	inc hl
	jr lp_end
lp_none:
	ld (hl), UI_FONT_CH_3F
	inc hl
	jr lp_end
lp_part:
	ex de, hl
	call app_dec16
	ld a, UI_FONT_CH_2F
	ld (de), a
	inc de
	ld hl, (LIBS_TOTAL)
	call app_dec16
	ex de, hl
lp_end:
	ld (hl), 0
	ret
#endif

libs_key:
	cp KEY_UP
	jr z, libs_up
	cp KEY_DOWN
	jr z, libs_down
	cp KEY_ENTER
	jr z, libs_open
	cp KEY_F1
	jr z, libs_open
	cp KEY_F2
	jr z, libs_run
	cp KEY_F3
	jr z, libs_info
	cp KEY_CLEAR
	ld a, SCR_STAY
	ret nz
	ld a, SCR_POP
	ret

libs_up:
	inc hl
	ld a, (hl)
	or a
	jr z, libs_stay
	dec a
	ld (hl), a
	inc hl
	cp (hl)
	jr nc, libs_moved
	ld (hl), a
	jr libs_moved

libs_down:
	inc hl
	ld a, (hl)
	cp STAGE_COUNT
	jr nc, libs_stay
	inc a
	ld (hl), a
	inc hl
	ld c, a
	ld a, (hl)
	add a, LIBS_VISIBLE
	cp c
	jr z, libs_scroll
	jr nc, libs_moved
libs_scroll:
	ld a, c
	sub LIBS_VISIBLE - 1
	ld (hl), a
libs_moved:
	ld a, SCR_REDRAW
	ret
libs_stay:
	ld a, SCR_STAY
	ret

libs_open:
	inc hl
	ld a, (hl)
	ld b, NOTICE_PROOFS
	ld c, 0
	cp STAGE_COUNT
	jr nc, libs_notice
	ld b, NOTICE_LIST
	ld c, a
	jr libs_notice

libs_run:
	inc hl
	ld a, (hl)
	ld b, NOTICE_PICK
	cp STAGE_COUNT
	jr nc, libs_notice_plain
	ld b, NOTICE_VERIFY
libs_notice_plain:
	ld c, 0
libs_notice:
	ld a, KIND_NOTICE
	jp screen_push_param

libs_info:
#if STAGE_COUNT > 1
	inc hl
	ld a, (hl)
	cp STAGE_COUNT
	jr nc, libs_program
	ld b, a
	ld a, KIND_STAGE
	jp screen_push
#endif
libs_program:
	ld a, KIND_ABOUT
	ld b, 0
	jp screen_push
