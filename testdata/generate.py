#!/usr/bin/env python3
"""mosty のテスト用 Excel ファイルを生成する。

使い方:
    pip install xlsxwriter
    python3 testdata/generate.py

各ファイルの内容と期待される結果は testdata/README.md を参照。
学生の氏名・学生証番号はすべて架空のものである。
"""

import re
import shutil
import zipfile
from pathlib import Path

import xlsxwriter

OUT = Path(__file__).resolve().parent

# (学生証番号, 氏名)
STUDENTS = [
    (1234001, "京都 太郎"),
    (1234002, "京産 花子"),
    (1234003, "山田 一郎"),
    (1234004, "佐藤 次郎"),
    (1234005, "鈴木 三子"),
]
KADAI1 = [8, 10, 7, 9, 6]
KADAI2 = [9, 8, 10, 5, 7]
KADAI_TOTAL = [a + b for a, b in zip(KADAI1, KADAI2)]
SHIKEN = [85, 92, 70, 64, 77]
HAITEN_KADAI = 40
HAITEN_SHIKEN = 60


class F:
    """数式セル。cached は Excel が保存する計算結果（キャッシュ値）。"""

    def __init__(self, formula, cached=0):
        self.formula = formula
        self.cached = cached


def write_book(path, sheets, names=None):
    """sheets: [(シート名, {(row, col): 値 または F})]。row, col は 0 始まり。"""
    wb = xlsxwriter.Workbook(str(path))
    for sheet_name, cells in sheets:
        ws = wb.add_worksheet(sheet_name)
        for (r, c), v in sorted(cells.items()):
            if isinstance(v, F):
                ws.write_formula(r, c, v.formula, None, v.cached)
            elif v is not None:
                ws.write(r, c, v)
    for name, ref in (names or {}).items():
        wb.define_name(name, ref)
    wb.close()


def row(cells, r, values, start_col=0):
    for i, v in enumerate(values):
        cells[(r, start_col + i)] = v


# ---------------------------------------------------------------------------
# 共通のシート
# ---------------------------------------------------------------------------


def meibo():
    """名簿: 行 0 が見出し、行 1〜5 が学生。A 列に学生証番号、B 列に氏名。"""
    cells = {}
    row(cells, 0, ["学生証番号", "氏名"])
    for i, (sid, name) in enumerate(STUDENTS):
        row(cells, 1 + i, [sid, name])
    return cells


def kadai():
    """課題: 行 0 がタイトル、行 2 が見出し、行 3〜7 が学生、行 9 が平均。"""
    cells = {}
    row(cells, 0, ["課題の成績"])
    row(cells, 2, ["学生証番号", "氏名", "課題1", "課題2", "合計"])
    for i, (sid, name) in enumerate(STUDENTS):
        r = 3 + i
        row(cells, r, [sid, name, KADAI1[i], KADAI2[i],
                       F(f"=SUM(C{r + 1}:D{r + 1})", KADAI_TOTAL[i])])
    row(cells, 9, ["平均", None, None, None,
                   F("=AVERAGE(E4:E8)", sum(KADAI_TOTAL) / len(KADAI_TOTAL))])
    return cells


def shiken(ids=None):
    """試験: 行 0 が見出し、行 1〜5 が学生。

    学生証番号の揺れを確認するため、一部を文字列（前後に空白を含むものもある）で格納する。
    """
    ids = ids or ["1234001", 1234002, "1234003", "1234004　", 1234005]
    cells = {}
    row(cells, 0, ["学生証番号", "氏名", "得点"])
    for i, (_, name) in enumerate(STUDENTS):
        row(cells, 1 + i, [ids[i], name, SHIKEN[i]])
    return cells


def haiten():
    """配点: 学生シートではないシート。"""
    cells = {}
    row(cells, 0, ["項目", "配点"])
    row(cells, 1, ["課題", HAITEN_KADAI])
    row(cells, 2, ["試験", HAITEN_SHIKEN])
    return cells


def kadai_score(i):
    return KADAI_TOTAL[i] * HAITEN_KADAI / 20 if 0 <= i < len(STUDENTS) else 0


def shiken_score(i):
    return SHIKEN[i] * HAITEN_SHIKEN / 100 if 0 <= i < len(STUDENTS) else 0


def saishu(kadai_rows=None, shiken_rows=None, name_rows=None):
    """最終成績: 行 0 が見出し、行 1〜5 が学生、行 7 が平均。

    学生証番号・氏名は名簿を参照する数式。*_rows は各学生が参照する行の学生番号（0〜4）で、
    ずれを作るときに変更する（範囲外の値は学生のいない行を指す）。
    """
    n = len(STUDENTS)
    kadai_rows = kadai_rows or list(range(n))
    shiken_rows = shiken_rows or list(range(n))
    name_rows = name_rows or list(range(n))
    cells = {}
    row(cells, 0, ["学生証番号", "氏名", "課題", "試験", "合計", "課題合計(確認)"])
    for i, (sid, _) in enumerate(STUDENTS):
        r = i + 2  # Excel の行番号
        k, s, nm = kadai_rows[i], shiken_rows[i], name_rows[i]
        ks, ss = kadai_score(k), shiken_score(s)
        row(cells, 1 + i, [
            F(f"=名簿!A{r}", sid),
            F(f"=名簿!B{nm + 2}", STUDENTS[nm][1]),
            F(f"=課題!E{k + 4}*配点!$B$2/20", ks),
            F(f"=試験!C{s + 2}*配点!$B$3/100", ss),
            F(f"=C{r}+D{r}", ks + ss),
            # 検索関数の検索範囲は照合しない（6.4）
            F(f"=VLOOKUP(A{r},課題!$A$4:$E$8,5,FALSE)", KADAI_TOTAL[i]),
        ])
    avg_s = sum(shiken_score(i) for i in range(n)) / n
    row(cells, 7, [
        "平均", None, None,
        # 学生行ではない行からの複数行範囲の参照は照合しない
        F("=AVERAGE(試験!C2:C6)*配点!$B$3/100", avg_s),
    ])
    return cells


def standard_sheets(**kwargs):
    return [
        ("名簿", meibo()),
        ("課題", kadai()),
        ("試験", kwargs.pop("shiken", None) or shiken()),
        ("配点", haiten()),
        ("最終成績", saishu(**kwargs)),
    ]


# ---------------------------------------------------------------------------
# 各テストファイル
# ---------------------------------------------------------------------------


def gen_valid():
    write_book(OUT / "valid.xlsx", standard_sheets())


def gen_valid_xlsm():
    """valid.xlsx と同じ内容の .xlsm（マクロは含まない）。"""
    src = OUT / "valid.xlsx"
    dst = OUT / "valid.xlsm"
    with zipfile.ZipFile(src) as zin, zipfile.ZipFile(dst, "w", zipfile.ZIP_DEFLATED) as zout:
        for item in zin.infolist():
            data = zin.read(item.filename)
            if item.filename == "[Content_Types].xml":
                data = data.replace(
                    b"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
                    b"application/vnd.ms-excel.sheet.macroEnabled.main+xml",
                )
            zout.writestr(item, data)


def gen_id_mismatch():
    # 学生 2（1234003）以降の課題の参照が 1 行下にずれている。
    # 学生 4（1234005）は課題の行 8（空行）を参照する。
    write_book(OUT / "id_mismatch.xlsx", standard_sheets(kadai_rows=[0, 1, 3, 4, 5]))


def gen_name_mismatch():
    # 学生証番号は正しく名簿を参照しているが、学生 2（1234003）の氏名だけ 1 行下を参照している。
    write_book(OUT / "name_mismatch.xlsx", standard_sheets(name_rows=[0, 1, 3, 3, 4]))


def gen_multi_student_range():
    sheets = standard_sheets()
    final = sheets[4][1]
    final[(0, 6)] = "確認用"
    # 学生 1（1234002, 行 2）: 単一行の範囲（正しい行）→ 問題なし
    final[(2, 6)] = F("=SUM(課題!C5:D5)", KADAI_TOTAL[1])
    # 学生 1（1234002, 行 2）: 複数の学生行を含む範囲 → 警告
    final[(2, 7)] = F("=SUM(課題!E4:E8)", sum(KADAI_TOTAL))
    # 学生 2（1234003, 行 3）: 列全体の参照 → 警告
    final[(3, 7)] = F("=MAX(試験!C:C)", max(SHIKEN))
    write_book(OUT / "multi_student_range.xlsx", sheets)


def gen_duplicated_id():
    # 試験シートで 1234004 の行の学生証番号が 1234003 になっている。
    write_book(OUT / "duplicated_id.xlsx", standard_sheets(
        shiken=shiken(ids=[1234001, 1234002, 1234003, 1234003, 1234005])))


def gen_layout():
    """氏名が学生証番号の左にあり、表が A1 から始まらず、途中に空行・見出し行があるシート。"""
    kadai_sheet = "課題 (前半)"
    cells = {}
    # 行 2 から、列 1（B 列）から始まる。A 列は空。
    row(cells, 2, ["氏名", "学生証番号", "課題1"], start_col=1)
    layout_rows = {0: 3, 1: 4, 2: 7, 3: 8, 4: 9}  # 学生番号 → 行インデックス
    for i, r in layout_rows.items():
        sid, name = STUDENTS[i]
        row(cells, r, [name, sid, KADAI1[i]], start_col=1)
    # 行 5 は空行、行 6 は表の途中の見出し行
    row(cells, 6, ["（再履修者）"], start_col=1)

    final = {}
    row(final, 0, ["氏名", "学生証番号", "課題1"])
    for i, (sid, name) in enumerate(STUDENTS):
        r = layout_rows[i] + 1  # Excel の行番号
        row(final, 1 + i, [name, sid, F(f"='{kadai_sheet}'!$D${r}", KADAI1[i])])
    write_book(OUT / "layout.xlsx", [
        (kadai_sheet, cells),
        ("最終成績", final),
    ])


def gen_id_formula_nocache():
    """学生証番号が数式で、キャッシュ値が保存されていないファイル。"""
    n = len(STUDENTS)
    middle = {}
    row(middle, 0, ["学生証番号", "氏名"])
    final = {}
    row(final, 0, ["学生証番号", "氏名", "試験"])
    for i in range(n):
        r = i + 2
        row(middle, 1 + i, [F(f"=名簿!A{r}"), F(f"=名簿!B{r}")])
        row(final, 1 + i, [F(f"=中間!A{r}"), F(f"=中間!B{r}"), F(f"=試験!C{r}")])
    # 学生 3: 単純な参照ではない数式 → 学生証番号を求められない
    final[(4, 0)] = F('=TEXT(名簿!A5,"0")')
    # 学生 4: 循環参照 → 10 段で打ち切る
    middle[(5, 0)] = F("=最終成績!A6")
    write_book(OUT / "id_formula_nocache.xlsx", [
        ("名簿", meibo()),
        ("中間", middle),
        ("試験", shiken()),
        ("最終成績", final),
    ])
    strip_cached_values(OUT / "id_formula_nocache.xlsx")


def strip_cached_values(path):
    """数式セルのキャッシュ値（<v>）を取り除く。"""
    tmp = path.with_suffix(".tmp")
    pattern = re.compile(rb'<c ([^>]*?)(?: t="[a-z]+")?>(<f>.*?</f>)<v>.*?</v></c>')
    with zipfile.ZipFile(path) as zin, zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
        for item in zin.infolist():
            data = zin.read(item.filename)
            if item.filename.startswith("xl/worksheets/"):
                data = pattern.sub(rb"<c \1>\2</c>", data)
            zout.writestr(item, data)
    shutil.move(tmp, path)


def gen_prefixed_id():
    """名簿の学生証番号の先頭に英字が付いているファイル。"""
    roster = {}
    row(roster, 0, ["学生証番号", "氏名", "学年"])
    grades = [1, 2, 1, 3, 2]
    for i, (sid, name) in enumerate(STUDENTS):
        row(roster, 1 + i, [f"g{sid}", name, grades[i]])
    final = {}
    row(final, 0, ["学生証番号", "氏名", "学年"])
    for i, (sid, name) in enumerate(STUDENTS):
        src = i + 1 if i == 3 else i  # 学生 3（1234004）だけ 1 行下を参照
        final_row = [sid, name, F(f"=名簿!C{src + 2}", grades[src])]
        row(final, 1 + i, final_row)
    write_book(OUT / "prefixed_id.xlsx", [("名簿", roster), ("最終成績", final)])


def gen_unsupported_refs():
    """静的に参照先が決まらない参照を含むファイル。"""
    sheets = standard_sheets()
    final = sheets[4][1]
    final[(0, 6)] = "確認用"
    final[(1, 6)] = F('=INDIRECT("課題!E"&(ROW()+2))', KADAI_TOTAL[0])
    final[(2, 6)] = F("=OFFSET(課題!$E$4,1,0)", KADAI_TOTAL[1])
    final[(3, 6)] = F("=SUM(課題合計)", sum(KADAI_TOTAL))
    write_book(OUT / "unsupported_refs.xlsx", sheets,
               names={"課題合計": "=課題!$E$4:$E$8"})


def gen_stale():
    """設定ファイル（mosty.stale.xlsx.json5）を作った後に変更されたファイル。"""
    sheets = standard_sheets()
    # 名簿に学生を 1 人追加（行 6）
    row(sheets[0][1], 6, [1234006, "高橋 四郎"])
    # シート「小テスト」を追加
    quiz = {}
    row(quiz, 0, ["学生証番号", "氏名", "小テスト1"])
    for i, (sid, name) in enumerate(STUDENTS):
        row(quiz, 1 + i, [sid, name, 5])
    sheets.append(("小テスト", quiz))
    write_book(OUT / "stale.xlsx", sheets)


def main():
    gen_valid()
    gen_valid_xlsm()
    gen_id_mismatch()
    gen_name_mismatch()
    gen_multi_student_range()
    gen_duplicated_id()
    gen_layout()
    gen_id_formula_nocache()
    gen_prefixed_id()
    gen_unsupported_refs()
    gen_stale()


if __name__ == "__main__":
    main()
