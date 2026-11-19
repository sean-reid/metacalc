$( Real proofs for fm tests: a closed subset of the propositional and predicate libraries. $)
$c ( ) -> -. wff |- <-> /\ \/ A. setvar class = T. F. E. F/ [ / ] e. E* E! $.
$v ph ps ch th ta et ze x A B y z w v u t s $.
f.0 $f wff ph $.
f.1 $f wff ps $.
f.2 $f wff ch $.
f.3 $f wff th $.
f.4 $f wff ta $.
f.5 $f wff et $.
f.6 $f wff ze $.
f.7 $f setvar x $.
f.8 $f class A $.
f.9 $f class B $.
f.10 $f setvar y $.
f.11 $f setvar z $.
f.12 $f setvar w $.
f.13 $f setvar v $.
f.14 $f setvar u $.
f.15 $f setvar t $.
f.16 $f setvar s $.
${
  wn $a wff -. ph $.
$}
${
  wi $a wff ( ph -> ps ) $.
$}
${
  ax-mp.e1 $e |- ph $.
  ax-mp.e2 $e |- ( ph -> ps ) $.
  ax-mp $a |- ps $.
$}
${
  ax-1 $a |- ( ph -> ( ps -> ph ) ) $.
$}
${
  ax-2 $a |- ( ( ph -> ( ps -> ch ) ) -> ( ( ph -> ps ) -> ( ph -> ch ) ) ) $.
$}
${
  ax-3 $a |- ( ( -. ph -> -. ps ) -> ( ps -> ph ) ) $.
$}
${
  mp2.e1 $e |- ph $.
  mp2.e2 $e |- ps $.
  mp2.e3 $e |- ( ph -> ( ps -> ch ) ) $.
  mp2 $p |- ch $= ( wi ax-mp ) BCEABCGDFHH $.
$}
${
  a1i.e1 $e |- ph $.
  a1i $p |- ( ps -> ph ) $= ( wi ax-1 ax-mp ) ABADCABEF $.
$}
${
  a2i.e1 $e |- ( ph -> ( ps -> ch ) ) $.
  a2i $p |- ( ( ph -> ps ) -> ( ph -> ch ) ) $= ( wi ax-2 ax-mp ) ABCEEABEACEED
    ABCFG $.
$}
${
  mpd.e1 $e |- ( ph -> ps ) $.
  mpd.e2 $e |- ( ph -> ( ps -> ch ) ) $.
  mpd $p |- ( ph -> ch ) $= ( wi a2i ax-mp ) ABFACFDABCEGH $.
$}
${
  imim2i.e1 $e |- ( ph -> ps ) $.
  imim2i $p |- ( ( ch -> ph ) -> ( ch -> ps ) ) $= ( wi a1i a2i ) CABABECDFG $.
$}
${
  syl.e1 $e |- ( ph -> ps ) $.
  syl.e2 $e |- ( ps -> ch ) $.
  syl $p |- ( ph -> ch ) $= ( wi a1i mpd ) ABCDBCFAEGH $.
$}
${
  id $p |- ( ph -> ph ) $= ( wi ax-1 mpd ) AAABZAAACAECD $.
$}
${
  idd $p |- ( ph -> ( ps -> ps ) ) $= ( wi id a1i ) BBCABDE $.
$}
${
  a1d.e1 $e |- ( ph -> ps ) $.
  a1d $p |- ( ph -> ( ch -> ps ) ) $= ( wi ax-1 syl ) ABCBEDBCFG $.
$}
${
  a2d.e1 $e |- ( ph -> ( ps -> ( ch -> th ) ) ) $.
  a2d $p |- ( ph -> ( ( ps -> ch ) -> ( ps -> th ) ) ) $= ( wi ax-2 syl ) ABCDF
    FBCFBDFFEBCDGH $.
$}
${
  sylcom.e1 $e |- ( ph -> ( ps -> ch ) ) $.
  sylcom.e2 $e |- ( ps -> ( ch -> th ) ) $.
  sylcom $p |- ( ph -> ( ps -> th ) ) $= ( wi a2i syl ) ABCGBDGEBCDFHI $.
$}
${
  syl5com.e1 $e |- ( ph -> ps ) $.
  syl5com.e2 $e |- ( ch -> ( ps -> th ) ) $.
  syl5com $p |- ( ph -> ( ch -> th ) ) $= ( a1d sylcom ) ACBDABCEGFH $.
$}
${
  com12.e1 $e |- ( ph -> ( ps -> ch ) ) $.
  com12 $p |- ( ps -> ( ph -> ch ) ) $= ( id syl5com ) BBACBEDF $.
$}
${
  syl5.e1 $e |- ( ph -> ps ) $.
  syl5.e2 $e |- ( ch -> ( ps -> th ) ) $.
  syl5 $p |- ( ch -> ( ph -> th ) ) $= ( syl5com com12 ) ACDABCDEFGH $.
$}
${
  syl6.e1 $e |- ( ph -> ( ps -> ch ) ) $.
  syl6.e2 $e |- ( ch -> th ) $.
  syl6 $p |- ( ph -> ( ps -> th ) ) $= ( wi a1i sylcom ) ABCDECDGBFHI $.
$}
${
  pm2.27 $p |- ( ph -> ( ( ph -> ps ) -> ps ) ) $= ( wi id com12 ) ABCZABFDE $.
$}
${
  mpdd.e1 $e |- ( ph -> ( ps -> ch ) ) $.
  mpdd.e2 $e |- ( ph -> ( ps -> ( ch -> th ) ) ) $.
  mpdd $p |- ( ph -> ( ps -> th ) ) $= ( wi a2d mpd ) ABCGBDGEABCDFHI $.
$}
${
  syld.e1 $e |- ( ph -> ( ps -> ch ) ) $.
  syld.e2 $e |- ( ph -> ( ch -> th ) ) $.
  syld $p |- ( ph -> ( ps -> th ) ) $= ( wi a1d mpdd ) ABCDEACDGBFHI $.
$}
${
  con4 $p |- ( ( -. ph -> -. ps ) -> ( ps -> ph ) ) $= ( ax-3 ) ABC $.
$}
${
  con4i.e1 $e |- ( -. ph -> -. ps ) $.
  con4i $p |- ( ps -> ph ) $= ( wn wi con4 ax-mp ) ADBDEBAECABFG $.
$}
${
  con4d.e1 $e |- ( ph -> ( -. ps -> -. ch ) ) $.
  con4d $p |- ( ph -> ( ch -> ps ) ) $= ( wn wi con4 syl ) ABECEFCBFDBCGH $.
$}
${
  mt4.e1 $e |- ph $.
  mt4.e2 $e |- ( -. ps -> -. ph ) $.
  mt4 $p |- ps $= ( con4i ax-mp ) ABCBADEF $.
$}
${
  mt4d.e1 $e |- ( ph -> ps ) $.
  mt4d.e2 $e |- ( ph -> ( -. ch -> -. ps ) ) $.
  mt4d $p |- ( ph -> ch ) $= ( con4d mpd ) ABCDACBEFG $.
$}
${
  pm2.21i.e1 $e |- -. ph $.
  pm2.21i $p |- ( ph -> ps ) $= ( wn a1i con4i ) BAADBDCEF $.
$}
${
  pm2.21d.e1 $e |- ( ph -> -. ps ) $.
  pm2.21d $p |- ( ph -> ( ps -> ch ) ) $= ( wn a1d con4d ) ACBABECEDFG $.
$}
${
  pm2.21 $p |- ( -. ph -> ( ph -> ps ) ) $= ( wn id pm2.21d ) ACZABFDE $.
$}
${
  jarli.e1 $e |- ( ( ph -> ps ) -> ch ) $.
  jarli $p |- ( -. ph -> ch ) $= ( wn wi pm2.21 syl ) AEABFCABGDH $.
$}
${
  pm2.18d.e1 $e |- ( ph -> ( -. ps -> ps ) ) $.
  pm2.18d $p |- ( ph -> ps ) $= ( id wn pm2.21 sylcom mt4d ) AABADABEBAEZCBIFGH
    $.
$}
${
  pm2.18 $p |- ( ( -. ph -> ph ) -> ph ) $= ( wn wi id pm2.18d ) ABACZAFDE $.
$}
${
  notnotr $p |- ( -. -. ph -> ph ) $= ( wn pm2.18 jarli ) ABAAACD $.
$}
${
  notnotri.e1 $e |- -. -. ph $.
  notnotri $p |- ph $= ( wn pm2.21i mt4 ) ACZCZABFGCBDE $.
$}
${
  con2d.e1 $e |- ( ph -> ( ps -> -. ch ) ) $.
  con2d $p |- ( ph -> ( ch -> -. ps ) ) $= ( wn notnotr syl5 con4d ) ABEZCIEBAC
    EBFDGH $.
$}
${
  mt2d.e1 $e |- ( ph -> ch ) $.
  mt2d.e2 $e |- ( ph -> ( ps -> -. ch ) ) $.
  mt2d $p |- ( ph -> -. ps ) $= ( wn con2d mpd ) ACBFDABCEGH $.
$}
${
  nsyl3.e1 $e |- ( ph -> -. ps ) $.
  nsyl3.e2 $e |- ( ch -> ps ) $.
  nsyl3 $p |- ( ch -> -. ph ) $= ( wn wi a1i mt2d ) CABEABFGCDHI $.
$}
${
  con2i.e1 $e |- ( ph -> -. ps ) $.
  con2i $p |- ( ps -> -. ph ) $= ( id nsyl3 ) ABBCBDE $.
$}
${
  nsyl2.e1 $e |- ( ph -> -. ps ) $.
  nsyl2.e2 $e |- ( -. ch -> ps ) $.
  nsyl2 $p |- ( ph -> ch ) $= ( wn nsyl3 con4i ) CAABCFDEGH $.
$}
${
  notnot $p |- ( ph -> -. -. ph ) $= ( wn id con2i ) ABZAECD $.
$}
${
  con1d.e1 $e |- ( ph -> ( -. ps -> ch ) ) $.
  con1d $p |- ( ph -> ( -. ch -> ps ) ) $= ( wn notnot syl6 con4d ) ABCEZABECIE
    DCFGH $.
$}
${
  con1i.e1 $e |- ( -. ph -> ps ) $.
  con1i $p |- ( -. ps -> ph ) $= ( wn id nsyl2 ) BDZBAGECF $.
$}
${
  con3d.e1 $e |- ( ph -> ( ps -> ch ) ) $.
  con3d $p |- ( ph -> ( -. ch -> -. ps ) ) $= ( wn notnotr syl5 con1d ) ABEZCIE
    BACBFDGH $.
$}
${
  con3rr3.e1 $e |- ( ph -> ( ps -> ch ) ) $.
  con3rr3 $p |- ( -. ch -> ( ph -> -. ps ) ) $= ( wn con3d com12 ) ACEBEABCDFG
    $.
$}
${
  pm3.2im $p |- ( ph -> ( ps -> -. ( ph -> -. ps ) ) ) $= ( wn wi pm2.27 con2d
    ) AABCZDBAGEF $.
$}
${
  impi.e1 $e |- ( ph -> ( ps -> ch ) ) $.
  impi $p |- ( -. ( ph -> -. ps ) -> ch ) $= ( wn wi con3rr3 con1i ) CABEFABCDG
    H $.
$}
${
  expi.e1 $e |- ( -. ( ph -> -. ps ) -> ch ) $.
  expi $p |- ( ph -> ( ps -> ch ) ) $= ( wn wi pm3.2im syl6 ) ABABEFECABGDH $.
$}
${
  simprim $p |- ( -. ( ph -> -. ps ) -> ps ) $= ( idd impi ) ABBABCD $.
$}
${
  pm2.61d.e1 $e |- ( ph -> ( ps -> ch ) ) $.
  pm2.61d.e2 $e |- ( ph -> ( -. ps -> ch ) ) $.
  pm2.61d $p |- ( ph -> ch ) $= ( wn con1d syld pm2.18d ) ACACFBCABCEGDHI $.
$}
${
  pm2.61d1.e1 $e |- ( ph -> ( ps -> ch ) ) $.
  pm2.61d1.e2 $e |- ( -. ps -> ch ) $.
  pm2.61d1 $p |- ( ph -> ch ) $= ( wn wi a1i pm2.61d ) ABCDBFCGAEHI $.
$}
${
  ja.e1 $e |- ( -. ph -> ch ) $.
  ja.e2 $e |- ( ps -> ch ) $.
  ja $p |- ( ( ph -> ps ) -> ch ) $= ( wi imim2i pm2.61d1 ) ABFACBCAEGDH $.
$}
${
  pm2.01 $p |- ( ( ph -> -. ph ) -> -. ph ) $= ( wn id ja ) AABZEECZFD $.
$}
${
  pm2.01i.e1 $e |- ( ph -> -. ph ) $.
  pm2.01i $p |- -. ph $= ( wn wi pm2.01 ax-mp ) AACZDGBAEF $.
$}
${
  pm2.65i.e1 $e |- ( ph -> ps ) $.
  pm2.65i.e2 $e |- ( ph -> -. ps ) $.
  pm2.65i $p |- -. ph $= ( nsyl3 pm2.01i ) AABADCEF $.
$}
${
  mto.e1 $e |- -. ps $.
  mto.e2 $e |- ( ph -> ps ) $.
  mto $p |- -. ph $= ( wn a1i pm2.65i ) ABDBEACFG $.
$}
${
  mt3.e1 $e |- -. ps $.
  mt3.e2 $e |- ( -. ph -> ps ) $.
  mt3 $p |- ph $= ( wn mto notnotri ) AAEBCDFG $.
$}
${
  wb $a wff ( ph <-> ps ) $.
$}
${
  df-bi $a |- -. ( ( ( ph <-> ps ) -> -. ( ( ph -> ps ) -> -. ( ps -> ph ) ) )
    -> -. ( -. ( ( ph -> ps ) -> -. ( ps -> ph ) ) -> ( ph <-> ps ) ) ) $.
$}
${
  impbi $p |- ( ( ph -> ps ) -> ( ( ps -> ph ) -> ( ph <-> ps ) ) ) $= ( wi wb
    wn df-bi simprim ax-mp expi ) ABCZBACZABDZLJKECEZCZMLCZECEOABFNOGHI $.
$}
${
  dfbi1 $p |- ( ( ph <-> ps ) <-> -. ( ( ph -> ps ) -> -. ( ps -> ph ) ) ) $= (
    wb wi wn df-bi impbi con3rr3 mt3 ) ABCZABDBADEDEZCZJKDZKJDZEDABFMNLJKGHI $.
$}
${
  wal $a wff A. x ph $.
$}
${
  ax-gen.e1 $e |- ph $.
  ax-gen $a |- A. x ph $.
$}
${
  mpg.e1 $e |- ( A. x ph -> ps ) $.
  mpg.e2 $e |- ph $.
  mpg $p |- ps $= ( wal ax-gen ax-mp ) ACFBACEGDH $.
$}
${
  ax-4 $a |- ( A. x ( ph -> ps ) -> ( A. x ph -> A. x ps ) ) $.
$}
${
  alim $p |- ( A. x ( ph -> ps ) -> ( A. x ph -> A. x ps ) ) $= ( ax-4 ) ABCD
    $.
$}
${
  al2im $p |- ( A. x ( ph -> ( ps -> ch ) ) -> ( A. x ph -> ( A. x ps -> A. x
    ch ) ) ) $= ( wi wal alim syl6 ) ABCEZEDFADFIDFBDFCDFEAIDGBCDGH $.
$}
${
  al2imi.e1 $e |- ( ph -> ( ps -> ch ) ) $.
  al2imi $p |- ( A. x ph -> ( A. x ps -> A. x ch ) ) $= ( wi wal al2im mpg ) AB
    CFFADGBDGCDGFFDABCDHEI $.
$}
${
  alimdh.e1 $e |- ( ph -> A. x ph ) $.
  alimdh.e2 $e |- ( ph -> ( ps -> ch ) ) $.
  alimdh $p |- ( ph -> ( A. x ps -> A. x ch ) ) $= ( wal wi al2imi syl ) AADGBD
    GCDGHEABCDFIJ $.
$}
${
  $d ph x $.
  ax-5 $a |- ( ph -> A. x ph ) $.
$}
${
  $d ph x $.
  alimdv.e1 $e |- ( ph -> ( ps -> ch ) ) $.
  alimdv $p |- ( ph -> ( A. x ps -> A. x ch ) ) $= ( ax-5 alimdh ) ABCDADFEG $.
$}
${
  bad $p |- ( ph -> ph ) $= f.0 f.1 ax-1 $.
$}
