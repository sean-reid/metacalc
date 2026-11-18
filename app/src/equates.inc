#define bcall(addr) rst 28h \ .dw addr
#define bjump(addr) call 50h \ .dw addr

_JForceCmdNoChar = 4027h
_PutC            = 4504h
_ClrLCDFull      = 4540h
_GetKey          = 4972h
curRow           = 844Bh
curCol           = 844Ch
_NewLine         = 452Eh
appBackUpScreen  = 9872h
