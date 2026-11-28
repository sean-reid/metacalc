; The LCD probe. Two questions about the calculator's LCD, answered on its screen.
;
; 1. How often is the controller busy when ui_blit polls? The probe sends the frame at UI_FRAME to the
; LCD with probe_blit (lcdblit.inc), ui_blit's own instruction sequence with a count added after
; each wait, once at 6 MHz and once at 15 MHz, and shows for the data writes and for the command
; writes how many were ready at the first poll, needed one more, two more, three or more, and how many
; busy reads there were in all. The speed set before each pass is put back after it, each write of
; port 20h with interrupts off; the screen shows what port 20h reads back after the write. A pass
; sends only the writes ui_blit makes and reads only the status port. The timer ports are not used:
; nothing here shows the OS leaves them alone.
; 2. Does the OS write the LCD or plotSScreen while an app waits in _GetKey? Three times the probe
; fills plotSScreen with a pattern, sends it to the LCD, draws a prompt in text row 7, reads the LCD
; back through the driver (the dummy read, then the bytes), calls _GetKey, and reads the LCD back
; again. The first read-back must match the pattern (shown as pre); the second is compared with the
; pattern in pixel rows 0 to 55 (lcd), plotSScreen the same (plt), and rows 56 to 63, the prompt's,
; with their state before the wait (r7). Each line shows the count of differing bytes and the pixel
; row and byte column, 0 to 11, of the first one, or r-- c-- for none. The three waits are a plain key,
; 2nd then a key, ALPHA then a key; the key to press is 5. The read-back writes the commands ui_blit writes and reads the data port.
;
; Screens, each ended by any key (ON is not a key). Pass 6 and 15: title, d01 and d23 with the data
; writes' counts of 0 and 1, 2 and 3 or more extra polls, dtot, c01, c23, ctot likewise for the
; command writes, in20 the byte port 20h read back. Case: key and the code _GetKey returned, pre, lcd,
; plt, r7. Counts are five digits, or three when a count cannot exceed 768.
;
; CLEAR at any wait ends the probe the way the last screen does.
;
; Memory: scratch in appData and the read-back in appBackUpScreen; the frame is plotSScreen and the
; canvas's variables are in saveSScreen, as in the app. The OS's text calls write textShadow, curRow
; and curCol; the probe sets curRow and curCol itself. All of these are left as last written. The exit
; sets graphDraw, as the app does because it writes plotSScreen, and goes to the home screen through
; _JForceCmdNoChar with interrupts on. The speed is the one the probe found; the contrast and the
; bank ports are not written. Interrupts are off for every write of port 20h, every blit and every
; read-back, and on at every wait. The OS's own calls run only at the speed found. The shadow
; registers are not touched.
;
; Built with REF_BLIT the probe calls ui_blit from ui/blit.inc instead of probe_blit, whose counters then
; stay zero; the emulator tests trace both builds.

VERSION      = 1
LCD_BUF      = appBackUpScreen
S_BASE       = appData
S_SPEED0     = S_BASE
S_IN20       = S_BASE + 1
S_PASS       = S_BASE + 2
S_KEY        = S_BASE + 3
S_WAITS      = S_BASE + 4
S_CASE       = S_BASE + 5
LCD_HIST_DATA  = S_BASE + 6
LCD_TOTAL_DATA = S_BASE + 14
LCD_HIST_CMD   = S_BASE + 16
LCD_TOTAL_CMD  = S_BASE + 24
S_PRE        = S_BASE + 26
S_LCD        = S_BASE + 30
S_PLT        = S_BASE + 34
S_R7         = S_BASE + 38
LCD_I        = S_BASE + 42
LCD_IFF      = S_BASE + 43
LCD_PTR      = S_BASE + 44
LCD_COL      = S_BASE + 46
LCD_R        = S_BASE + 47
LCD_C        = S_BASE + 48
P_TMP        = S_BASE + 49
P_TMP2       = S_BASE + 51
P_STR        = S_BASE + 52
S_T2         = S_BASE + 54
SNAP_LCD     = S_BASE + 55
S_SIZE       = 55 + ((UI_HEIGHT - 56) * UI_STRIDE)
S_COUNTERS   = LCD_HIST_DATA
COUNTERS_LEN = LCD_TOTAL_CMD + 2 - LCD_HIST_DATA
CASES        = 3
A_END        = appData + APP_DATA_SIZE

RAM_WITHIN(S_BASE, S_SIZE, appData, A_END, "lcdcheck's scratch runs past appData")
RAM_WITHIN(LCD_BUF, UI_FRAME_SIZE, $8000, $C000, "lcdcheck's read-back runs past bank B")
RAM_DISJOINT(S_BASE, S_SIZE, UI_VARS, UI_VARS_SIZE, "lcdcheck's scratch overlaps UI_VARS")
RAM_DISJOINT(S_BASE, S_SIZE, UI_FRAME, UI_FRAME_SIZE, "lcdcheck's scratch overlaps UI_FRAME")
RAM_DISJOINT(S_BASE, S_SIZE, LCD_BUF, UI_FRAME_SIZE, "lcdcheck's scratch overlaps its read-back")
RAM_DISJOINT(LCD_BUF, UI_FRAME_SIZE, UI_VARS, UI_VARS_SIZE, "lcdcheck's read-back overlaps UI_VARS")
RAM_DISJOINT(LCD_BUF, UI_FRAME_SIZE, UI_FRAME, UI_FRAME_SIZE, "lcdcheck's read-back overlaps UI_FRAME")

; Contract: entered with interrupts on, at the speed the OS runs apps at.
probe_start:
	ei
	ld hl,S_BASE
	ld de,S_BASE + 1
	ld bc,S_SIZE - 1
	ld (hl),0
	ldir
	in a,(20h)
	ld (S_SPEED0),a
	call fill_pattern
pass_loop:
	ld hl,S_COUNTERS
	ld de,S_COUNTERS + 1
	ld bc,COUNTERS_LEN - 1
	ld (hl),0
	ldir
	ld a,(S_PASS)
	call set_speed
	call BLIT_CALL
pass_blit_done:
	in a,(20h)
	ld (S_IN20),a
	ld a,(S_SPEED0)
	call set_speed
	call show_pass
	call wait_key
	ld a,(S_PASS)
	inc a
	ld (S_PASS),a
	cp 2
	jr nz,pass_loop

case_loop:
	call fill_pattern
	call BLIT_CALL
case_blit_done:
	ld a,7
	call at_row
	call prompt_ptr
	call putstr
	ld de,LCD_BUF
	call read_lcd
pre_read_done:
	ld ix,S_PRE
	ld hl,LCD_BUF
	ld de,PAT_LCD_BYTES
	call cmp_pat
	ld hl,LCD_BUF + PAT_LCD_BYTES
	ld de,SNAP_LCD
	ld bc,SNAP_BYTES
	ldir
	call wait_key
	ld (S_KEY),a
	ld de,LCD_BUF
	call read_lcd
post_read_done:
	ld ix,S_LCD
	ld hl,LCD_BUF
	ld de,PAT_LCD_BYTES
	call cmp_pat
	ld ix,S_PLT
	ld hl,UI_FRAME
	ld de,PAT_LCD_BYTES
	call cmp_pat
	ld ix,S_R7
	call cmp_snap
	call show_case
	call wait_key
	ld a,(S_CASE)
	inc a
	ld (S_CASE),a
	cp CASES
	jr nz,case_loop
leave:
	set graphDraw,(iy + graphFlags)
	bjump(_JForceCmdNoChar)

; Contract: A is 0 for 6 MHz or 1 for 15. Writes port 20h with interrupts off, and leaves them as
; they were. Destroys AF and C.
set_speed:
	ld c,a
	ld a,i
	jp pe,ss_on
	ld a,i
	jp pe,ss_on
	ld a,c
	out (20h),a
	ret
ss_on:
	di
	ld a,c
	out (20h),a
	ei
	ret

; Returns the key _GetKey gave in A, or leaves for CLEAR. An ON press is not a key: it is let go and cleared, and the wait goes on.
wait_key:
	ld hl,S_WAITS
	inc (hl)
key_again:
	ei
	bcall(_GetKey)
	bit onInterrupt,(iy + onFlags)
	jr nz,key_on
	cp kClear
	jp z,leave
	ret
key_on:
	call on_release_and_clear
	jr key_again

#include "onbreak.inc"
#include "lcdread.inc"
#include "lcdscreen.inc"

#ifdef REF_BLIT
#define BLIT_CALL ui_blit
#include "ui/blit.inc"
#else
#define BLIT_CALL probe_blit
#include "lcdblit.inc"
#endif
