/*-
 * Copyright 2024 David Michael Barr
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted providing that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY
 * DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT,
 * STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING
 * IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    let patch = ref_patch();
    let old = vec![0; 524288];
    let mut bspatch = Vec::with_capacity(524288);
    let mut new = Vec::with_capacity(524288);

    let mut group = c.benchmark_group("throughput");
    group.throughput(Throughput::Bytes(new.capacity() as u64));
    group.bench_function("memcpy", |b| {
        b.iter(|| {
            new.clear();
            new.extend(black_box(&old));
        })
    });
    group.bench_function("patch", |b| {
        b.iter(|| {
            new.clear();
            aehobak::patch(
                black_box(&old),
                black_box(&mut &*patch),
                black_box(&mut new),
            )
            .unwrap();
        })
    });
    group.bench_function("bspatch", |b| {
        bspatch.clear();
        aehobak::decode(black_box(&mut &*patch), black_box(&mut bspatch)).unwrap();
        b.iter(|| {
            new.clear();
            bsdiff::patch(
                black_box(&old),
                black_box(&mut bspatch.as_slice()),
                black_box(&mut new),
            )
            .unwrap();
        })
    });
    group.bench_function("decode-bspatch", |b| {
        b.iter(|| {
            new.clear();
            bspatch.clear();
            aehobak::decode(black_box(&mut &*patch), black_box(&mut bspatch)).unwrap();
            bsdiff::patch(
                black_box(&old),
                black_box(&mut bspatch.as_slice()),
                black_box(&mut new),
            )
            .unwrap();
        })
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

fn ref_patch() -> Box<[u8]> {
    let mut patch = Vec::new();
    patch.extend(PATCH_HEX_1);
    patch.resize(232 * 64, b'0');
    patch.extend(PATCH_HEX_2);
    patch.resize(9322 * 2, b'0');
    patch
        .chunks_exact(2)
        .map(|xx| u8::from_str_radix(std::str::from_utf8(xx).unwrap(), 16).unwrap())
        .collect()
}

const PATCH_HEX_1: &[u8] = b"\
AA0801D1047005AB15E05DD67114D7755DD7615DD6755CC67119D77118CB755D\
D7615CD7651DD7B15C97755DC7655DD6755CC7761D87715DC7755DC6655DC775\
1DC7711D47655C97715CC6B15DC77554C77554C77558D7715D97755D97755CD7\
7119C7711DC7755D87611CC7715DD7755DD77559D7711DD7515C47715DD5751D\
D7751DD7751DD7655DD6755DE6751DD7715CD7755CC7615CD7751DC7711CC775\
5DD7755DD7755CD6711DD7515C47715DD5755CC77559C77559C7755DD7715DD7\
755DD7755DC6615D97711D87715DD75711110411440505110445104410144141\
1111541151141051044540505004115450414510114404444450414511155110\
5515144140441011450404444015155040141110100544414441041410451111\
0441545550100444041101111111155051011111500544044444114551115055\
4550051141041111155140501411555404110441441141104544040411110444\
1151514504444404101140040144444044101110110144041011454400555555\
5555011511151550111515111041505045445444145400511554501510145150\
5410554510054114145141444044111410114455401454440154555504110441\
0411444004510550144544410441554440504441114150550451104441111544\
0444144101114154054441450110551550511514114404111110415410511111\
051104010878FCC307001313B90F00091AE66D1803C4BD0B001311F405001202\
1438D402001D0EBCC401000B7BC49901001A03423D010022022B620C00390D91\
480100079E081E03E4B008000A1752581D10D8B105001F032FA409001F94A308\
00462A66D60B954D0100024FF901009405D2480A0C37020052032B4C05000A6A\
DE0500156F9201B7C8010027014DA30A002138CCB40300141CD00A08001A063B\
EA07001803C6E0060016D0093CE4FF01001306F39C0800094534A505001602DD\
0E1B12EA7A040024292103003D0DEDA30900061EB501003001727E0400261269\
C004002014A4E80943A0DA03001310889606001C0363FC0A0027B06401001507\
7B8609196C8008000918E341180347A802001B133470040023711603003D0DED\
A30900061EB5010086014A96FF0800290D0A760100089DFB0800246E321A323F\
04001B0A49F20400090EA4ED0900169BC001002003418A0B00390A80E902001E\
2E9A1D3126480800290EC1684731BF4D0D001E08583D0F0008BDB90800231D40\
9205000A73A97208000D30250B000A0B88050018AC77060020356D6B0A0031CC\
620A00258E2B0E65BA06132C040021205C3903001D2E231903A144080055AA24\
0900295F4EC90AE3F3010043534A030043013C683D03002518ADFE03000AA14B\
06000B26BE00060095013405945001000C0F5307000B26BE000600950134323F\
9506000A48B20C001116CF1E06009610043417C36603000D171CC203000A03A8\
0700123A366C0200093E845402000925232B0AB5F43F0300097DD0E604001127\
9DCC0400231A9FB8150960FD04000E19C50B006A18CA4601000E16BA81050003\
BC8F04003906F352271DD0070016763605000ECDFA0600381AA73D0100382360\
010034BA7B0D002C14435B0B000996791201001C2058F70600ECB20D072F0D7C\
4D11030007861503008BC9E702001DFAE902005A16380C03000B1BF7C407000B\
A0A8EB08001324F9F506000A381FBB0400131A793F01001003B2D603001A21A2\
79010018151F472D2E53310100210EC92902001054820D003014D74407000217\
1607001E12CE4602001244172060080041037B3D090012441608E80B00370CBB\
E50300232A5B4E14310208000F5F4C750A00271825C303000AA14B06000B2646\
5607005301D73801003705603401000C0F5307000B26465607005301D7380100\
372F73B106000A48B20C00111647C9040054017A01371437B90400100E75F50A\
03DC4504000F3AF54B0400093E700E080009267AC50A0E01986201000E1271F2\
0E000B047E1902001B0494080011095E0903002063680A0001DEC001003C18DD\
260300200708C80B003906DDA7040010EA07020034166FC9050017A15C03001D\
7ED22762BC0A002514B9E70900091458CA09000A46DB9C0B0009549EAD0800AE\
4544010007C8440100895F63040007CC67040098293A04001D743C04006719A6\
BB010014120BC707000BA0352904001324E41E06000A3807960400131A916401\
001003B2D603001A21BB290200180E126D0D000705A70B002D2ED064210EC929\
02001054820D003014D744070002171607001E12CE4602001244172060080041\
037B3D090012441608E80B00370CBBE50300232A5B4E14310208000F51D9B502\
000312BF0D002F0F275704000AA14B06000B260E1F0600470187013705603401\
000C0F5307000B260E1F060047018701372F73B106000A48B20C0011167F0006\
004801CA380100371437B904000D1799DC01000A4D090200123AF4D20400093E\
CF6602000925722902000A2D998104000A6915C601000A6A147105000C1DCD75\
03000C0F609407001701F8AC050010085620213B340B000206C73B1825880100\
200750290A0039060B6A10FDB60C003218AAB7040006E9BD01002E39CC2762BC\
0A002514B9E70900091458CA09000A7BF6530200000000000000000000000000";

const PATCH_HEX_2: &[u8] = b"\
0000000000000000000000000000000000000000000000000000000000000007\
C4070061031C03031003180214031A0104021D02160303130302070C03030F04\
03081004030A0A04040B05080B0904090A030320010203080C03020F0611030E\
0A04040F04040F04040A0A04030E04030E03030A0A04040F04040A0604080408\
0C040F05130405058F040F0414050C0B03030E040317010402080C0405060107\
0104030905050A010402080A0303080A0103020E010302080507040703010110\
030D11022215045D04030F0413040C0B03030E04031703030A0E040706010703\
040B05080A03030A0A03040B0A03030E03030A050904070301010A04090A0304\
0B0F051105090106800E041703140315040311030D010C03040F02030C0B080B\
0C090C04054D091F031D0D110C0D0315041D03180403110503070C0303070C04\
1402140404070C0104030F0104030E030307010902030E02030D02030A0A0303\
0E04030E03030A0902030E02030A0604080D0E75230D09020F031378230D0902\
0F03137D230D094A3709090A0414040E04041503040F04040F01030423031402\
170303080D040E0F0315031601030407010B04040F02030F01041602220503F2\
8290941A08110B0A0B0E060705050B110C0209040F030C0F041A030417040407\
010B010203130303080E04030F03031103040A05010D03031804040D0F040985\
0313880313F71C140910031A0314021903021003140217030408020B0204080C\
04041803040F03130402D68F9D8C01191A08110B0A0B100B070A0B100C020904\
0F030C0F041A030417040407010B010203130303080E04030F03031103040A05\
010D03031804040D110F067903137C0313B9470B1009090F0103031604140319\
030211031402170304080E0403060D04031803040F0313040200000000000000";
