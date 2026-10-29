#!/bin/bash
# Synthetic speech clips: espeak-ng (ar) and flite slt (en). 16 kHz mono wav + .txt reference.
# Local tools only, nothing downloaded. Run: bash fixtures/make_audio_cases.sh
set -eu
D="$(cd "$(dirname "$0")" && pwd)/cases/audio"; mkdir -p "$D"; T=$(mktemp -d)
mk() { # name lang text
  printf '%s\n' "$3" > "$D/$1.txt"
  if [ "$2" = ar ]; then espeak-ng -v ar -s 175 -w "$T/$1.wav" "$3"
  else flite -voice slt -t "$3" -o "$T/$1.wav"; fi
  ffmpeg -v error -y -i "$T/$1.wav" -ar 16000 -ac 1 -c:a pcm_s16le "$D/$1.wav"
}
mk ar1 ar "اجتمع الفريق صباح يوم الأحد لمراجعة خطة المشروع الجديد. تقرر أن يرسل أحمد الفاتورة قبل نهاية الأسبوع، وأن تحضر سارة التقرير المالي يوم الثلاثاء."
mk ar2 ar "مرحبا بكم في خدمة تحويل الملفات. اختر الملف الذي تريده ثم اختر الصيغة الجديدة، وسيحفظ البرنامج النسخة الجديدة بجانب الملف الأصلي دون أن يغيره."
mk ar3 ar "تبدأ الرحلة من محطة القطار الرئيسية في تمام الساعة التاسعة صباحا. يرجى الوصول قبل ربع ساعة وإحضار التذكرة وبطاقة الهوية معكم."
mk en1 en "The team met on Monday morning to review the project plan. It was decided that Daniel will send the invoice before the end of the week, and that Maria will prepare the budget report on Wednesday."
mk en2 en "Welcome to the file converter. Choose the file you want, then choose the new format. The program saves the new copy next to the original file and never changes the original."
mk en3 en "The train leaves from the main station at nine o'clock in the morning. Please arrive fifteen minutes early and bring your ticket and a photo identity card with you."
for f in "$D"/*.wav; do printf '%s %s s\n' "$(basename "$f")" "$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$f")"; done
rm -rf "$T"
