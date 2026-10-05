### Повідомлення про помилки. Ядро повідомляє КОД; тут — слова до нього.
error-thicken-added-nothing = Пластина пішла всередину тіла й нічого не додала — задайте товщину більшу за нуль
error-draft-angle-zero = Ухил 0 градусів нічого не нахиляє — задайте кут, відмінний від нуля
error-torus-through-itself = Трубка не тонша за кільце — такий тор проходить крізь себе; задайте радіус трубки меншим за радіус кільця
error-array-of-one = Масив з однієї копії — це саме тіло; задайте дві копії або більше
### Підстановки { $name } несуть дані з ядра — їх не можна викидати, це не прикраса.

## Операція не вдалася в геометричному ядрі.
## Підказка в дужках — звичайна причина; вона заощаджує звернення до підтримки.

error-op-failed-extrude = Видавлювання не вдалося
error-op-failed-extrude-profile = Видавлювання не вдалося (перевірте профіль)
error-op-failed-extrude-contour = Видавлювання не вдалося (перевірте контур)
error-op-failed-revolve = Обертання не вдалося
error-op-failed-revolve-profile = Обертання не вдалося (перевірте профіль)
error-op-failed-revolve-axis = Обертання навколо датум-осі не вдалося (чи лежить вісь у площині ескізу?)
error-op-failed-sweep = Протягування не вдалося (чи стоїть профіль на початку шляху й приблизно перпендикулярно до нього?)
error-op-failed-loft = Лофт не вдався (перерізи мають бути замкненими й узгодженими)
error-op-failed-loft-boolean = Булева лофта з тілом не вдалася
error-op-failed-boolean = Булева не вдалася
error-op-failed-body-boolean = Булева тіл не вдалася (немає перетину, або тіла не пов’язані?)
error-op-failed-fillet = Заокруглення не вдалося (занадто великий радіус, або ребра?)
error-op-failed-fillet-var = Змінне заокруглення не вдалося (радіуси чи ребра?)
error-op-failed-chamfer = Фаска не вдалася (занадто великий розмір, або ребра?)
error-op-failed-chamfer-asym = Асиметрична фаска не вдалася (катет чи кут завеликі?)
error-op-failed-shell = Оболонка не вдалася (товщина чи грань?)
error-op-failed-shell-center = Оболонка по центру не вдалася (відступ чи грань?)
error-op-failed-draft = Ухил не вдався (чи можна нахилити цю грань на цей кут від цієї нейтралі?)
error-op-failed-push-face = Грань не зсувається (криволінійна грань, або самоперетин)
error-op-failed-remove-faces = Грані не можна видалити
error-op-failed-replace-faces = Поверхня не закрила отвір — грань не буде замінено
error-op-failed-copy-faces = Грань не копіюється окремою поверхнею
error-op-failed-offset-surface = Зміщення не будується: на цій відстані грань вивертається навиворіт або зникає - візьміть меншу відстань
error-op-failed-stitch = Аркуші не зшиваються: жодне ребро не збіглося — схоже, вони не торкаються
error-op-failed-mesh-recognise = Сітку не розпізнано: не вдалося побудувати жодної грані
error-op-failed-mesh-solid = Сітка не стала суцільним тілом: у ній немає жодного трикутника з площею
error-op-failed-trim = Обрізання не вдалося: поверхня й інструмент не перетинаються, або нічого відтинати
error-op-failed-thicken = Грань не потовщується (зміщення самоперетинається?)
error-op-failed-split-body = Площина не ріже тіло (проходить повз, або лежить на грані)
error-op-failed-split-faces = Площина не ділить жодної грані (проходить повз тіло)
error-op-failed-hole = Отвір не вдався (діаметри чи глибини?)
error-op-failed-holes = Отвори не вдалися (точки, діаметри чи глибини?)
error-op-failed-thread = Різьба не вдалася
error-op-failed-helix = Гвинтове протягування не вдалося
error-op-failed-auger = Шнек не вдався
error-op-failed-mirror = Дзеркало не вдалося
error-op-failed-mirror-plane = Дзеркало відносно площини не вдалося
error-op-failed-array = Масив не вдався
error-op-failed-move = Перенесення не вдалося
error-op-failed-transform = Перетворення не вдалося
error-op-failed-cylinder = Циліндр не вдався
error-op-failed-sphere = Куля не вдалася
error-op-failed-cone = Конус не вдався
error-op-failed-torus = Тор не вдався
error-op-failed-prism = Призма не вдалася
error-op-failed-fuse-profiles = Злиття контурів не вдалося
error-op-failed-place = Розміщення не вдалося

## Операції потрібне справжнє ядро OCCT (відповіла заглушка).
## Користувач зазвичай ніколи цього не бачить — це означає, що в збірці немає ядра.

error-kernel-required-extrude = Видавлювання потребує ядра OCCT
error-kernel-required-mesh-recognise = Сітку розпізнає лише ядро OCCT
error-kernel-required-mesh-solid = Сітку в суцільне тіло перетворює лише ядро OCCT
error-kernel-required-extrude-profile = Видавлювання потребує ядра OCCT
error-kernel-required-extrude-contour = Видавлювання потребує ядра OCCT
error-kernel-required-revolve = Обертання потребує ядра OCCT
error-kernel-required-revolve-profile = Обертання потребує ядра OCCT
error-kernel-required-revolve-axis = Обертання потребує ядра OCCT
error-kernel-required-sweep = Протягування потребує ядра OCCT
error-kernel-required-loft = Лофт потребує ядра OCCT
error-kernel-required-loft-boolean = Булева лофта потребує ядра OCCT
error-kernel-required-boolean = Булева потребує ядра OCCT
error-kernel-required-body-boolean = Булева тіл потребує ядра OCCT
error-kernel-required-fillet = Заокруглення потребує ядра OCCT
error-kernel-required-fillet-var = Змінне заокруглення потребує ядра OCCT
error-kernel-required-chamfer = Фаска потребує ядра OCCT
error-kernel-required-chamfer-asym = Асиметрична фаска потребує ядра OCCT
error-kernel-required-shell = Оболонка потребує ядра OCCT
error-kernel-required-shell-center = Оболонка по центру потребує ядра OCCT
error-kernel-required-draft = Ухил потребує ядра OCCT
error-kernel-required-push-face = Тягнення/штовхання грані потребує ядра OCCT
error-kernel-required-remove-faces = Видалення граней потребує ядра OCCT
error-kernel-required-replace-faces = Заміна грані поверхнею потребує ядра OCCT
error-kernel-required-copy-faces = Копіювання грані потребує ядра OCCT
error-kernel-required-offset-surface = Зміщення поверхні потребує ядра OCCT
error-kernel-required-thicken = Потовщення потребує ядра OCCT
error-kernel-required-split-body = Розділення тіла потребує ядра OCCT
error-kernel-required-split-faces = Розділення граней потребує ядра OCCT
error-kernel-required-hole = Отвір потребує ядра OCCT
error-kernel-required-holes = Отвори потребують ядра OCCT
error-kernel-required-thread = Різьба потребує ядра OCCT
error-kernel-required-helix = Гвинтове протягування потребує ядра OCCT
error-kernel-required-auger = Шнек потребує ядра OCCT
error-kernel-required-mirror = Дзеркало потребує ядра OCCT
error-kernel-required-mirror-plane = Дзеркало потребує ядра OCCT
error-kernel-required-array = Масив потребує ядра OCCT
error-kernel-required-move = Перенесення потребує ядра OCCT
error-kernel-required-transform = Перетворення потребує ядра OCCT
error-kernel-required-cylinder = Циліндр потребує ядра OCCT
error-kernel-required-sphere = Куля потребує ядра OCCT
error-kernel-required-cone = Конус потребує ядра OCCT
error-kernel-required-torus = Тор потребує ядра OCCT
error-kernel-required-prism = Призма потребує ядра OCCT
error-kernel-required-fuse-profiles = Злиття контурів потребує ядра OCCT
error-kernel-required-place = Розміщення потребує ядра OCCT

## Вхідні дані, яких бракує або які застаріли

error-source-body-not-built = Вихідне тіло не побудовано — спершу виправте операцію над цією
error-source-body-deleted = Те, на чому це було побудовано, видалено — виберіть інше тіло або видаліть цю операцію
error-body-in-pieces = Операція залишає деталь окремими шматками — деталь є одним тілом; зробіть так, щоб доданий елемент торкався тіла, або створіть нову деталь
error-body-in-one-piece = Тіло цільне — шматка, який можна зробити деталлю, немає
error-source-part-has-no-body = У вихідної деталі немає тіла
error-body-a-not-built = Тіло A не побудовано
error-body-b-not-built = Тіло B не побудовано
error-face-not-found = Грані більше немає у вихідному тілі — посилання застаріло
error-faces-not-found = Граней більше немає у вихідному тілі — посилання застаріли
error-profile-not-found = Профіль ескізу не знайдено
error-revolve-profile-crosses-axis = Профіль перетинає вісь обертання — жодна САПР цього не побудує. Притисніть профіль до осі (півперерізом: півколо замість кола) або відведіть вісь від профілю.
error-sweep-profile-missing = Профіль протягування не знайдено
error-sweep-path-missing = Шлях протягування не знайдено
error-no-isolated-points-for-holes = В ескізі немає ізольованих точок, у які можна поставити отвори
error-no-points-for-holes = Немає точок для розміщення отворів

## Опорні площини

error-cut-plane-deleted = Січну площину видалено — виберіть іншу або видаліть розділення
error-sketch-face-gone = Грані, на якій стоїть ескіз, більше немає: тіло, якому вона належала, видалено. Перенесіть ескіз на іншу грань або площину, або скасуйте видалення
error-sketch-plane-gone = Робочу площину, на якій стоїть ескіз, видалено. Перенесіть ескіз на іншу площину або грань, або скасуйте видалення
error-mirror-plane-deleted = Площину дзеркала видалено — виберіть іншу або видаліть дзеркало
error-split-plane-deleted = Площину розділення видалено — виберіть іншу або видаліть операцію
error-mirror-plane-unset = Площину дзеркала не задано — створіть дзеркальну деталь заново
error-zero-normal = Нормаль площини нульова — напряму не визначено

## Значення, що не мають сенсу

error-zero-thickness = Нульова товщина — пластини не буде
error-zero-push-distance = Нульова відстань — граню нікуди зсувати
error-broken-solid = Ядро повернуло непридатне тіло — операцію скасовано, деталь не змінилася. Таке зазвичай трапляється, коли грань межує із заокругленням або фаскою: спробуйте меншу відстань або перенесіть операцію до заокруглення в стрічці
error-split-piece-count = Площина тепер ріже тіло на { $got } частин замість { $want } — поверніть площину назад або створіть розділення заново
error-loft-needs-two-sections = Лофту потрібні принаймні два замкнені перерізи
error-draft-needs-faces = Ухилу потрібні грані, що нахиляються, і нейтральна грань
error-no-contours = Немає контурів для операції
error-all-edges-smooth = Кожне вибране ребро — гладке зчленування (межа заокруглення) — заокруглювати чи знімати фаску нема чого
error-fillet-radius-too-big = Заокруглення R{ $radius } не взялося: { $issues }{ $smooth }
# Одне ребро з цього списку. «приймає до» повідомляє найбільший радіус, який ПІДІЙДЕ.
error-fillet-edge-takes-up-to = ребро { $edge } (приймає до { $max })
error-fillet-edge-takes-none = ребро { $edge } (не приймає жодного радіуса — воно впирається в дотичне зчленування раніше зробленого заокруглення; приберіть це ребро або спершу заокругліть сусіднє)
error-fillet-smooth-skipped = ; гладких зчленувань автоматично пропущено: { $n }
error-fillet-edges-one-by-one = Заокруглення R{ $radius }: ці ребра беруться лише по одному — сусідні заокруглення перекриваються
error-chamfer-too-big = Фаска { $dist } мм не вдалася — катет більший за сторону
error-surface-does-not-close = Поверхня не збігається з отвором: ребер без пари — { $n }. Межі різняться — будуйте латку на тих самих ребрах, що обмежують грань, яку замінюють
error-push-face-on-sheet = Грань-поверхню не можна зсунути: це операція над суцільним тілом. Щоб надати поверхні товщини, скористайтеся «Потовщенням»
error-needs-solid-not-sheet = Це інструмент для суцільного тіла: до поверхні він не застосовується. Надайте поверхні товщини й працюйте з нею як зі звичайним тілом
error-draft-failed = Ухил { $angle }° на цих гранях не береться. Зазвичай заважає тонка стінка: після оболонки майже нічого схиляти — застосуйте ухил до оболонки або візьміть менший кут

## Різьба й шнеки

error-thread-rim-not-found = Обідок циліндра чи отвору (кругове ребро) не знайдено
error-thread-length-unset = Довжину різьби не задано
error-thread-pitch-too-small = Крок { $pitch } мм замалий
error-thread-too-many-turns = { $turns } витків — забагато: збільште крок або вкоротіть різьбу
error-thread-longer-than-face = Різьба довжиною { $length } мм довша за циліндр ({ $face } мм). Вкоротіть різьбу
error-thread-depth-too-deep = Глибина різьби { $depth } мм сягає радіуса { $radius } мм або виходить за нього: для Ø{ $dia } крок { $pitch } занадто грубий
error-thread-not-its-size = Різьба Ø{ $nominal } не підходить до грані Ø{ $face } — виберіть розмір за гранню або грань за розміром
warn-edges-dropped = Не вдалося взяти ребер: { $dropped } з { $asked }, їх залишено гострими; решту виконано — клацніть вузол двічі, щоб вибрати інші ребра або інший розмір
error-thread-removed-nothing = Різьба нічого не зняла ({ $before } -> { $after } мм³) — перевірте вибрану грань, крок і довжину
error-thread-failed = Різьба не побудувалася (перевірте крок, довжину й діаметр)
error-auger-rim-not-found = Обідок вала (кругове ребро) не знайдено
error-auger-bad-pitch-or-length = Крок і довжина шнека мають бути більшими за нуль
error-auger-outer-not-bigger = Зовнішній Ø{ $outer } шнека не більший за вал Ø{ $shaft }
error-auger-added-nothing = Стрічка шнека нічого не додала ({ $before } -> { $after } мм³) — перевірте зовнішній Ø і вибраний вал
error-auger-flight-failed = Стрічка шнека не побудувалася (перевірте крок, товщину й зовнішній діаметр)

## Ізоляція: деталь володіє своєю геометрією

error-body-only-in-part = Тіло можна побудувати лише всередині Деталі (Збірка тіл не тримає)
error-cross-component-input = Міжкомпонентне посилання не дозволене: вхід { $input } належить іншому компоненту
error-sketch-on-foreign-face = Вхід ескізу { $input } стоїть на грані тіла іншого компонента без зовнішнього посилання
error-sketch-face-ref-lost = Посилання на грань ескізу на тілі { $body } після перебудови не знайшлося за назвою — використано найближчий збіг, тому перевірте, куди потрапила операція

## Порожні результати

error-array-empty = Масив нічого не дав
error-empty-result = Результат — порожнє тіло
error-remove-faces-failed = Грані не можна видалити: { $why }

## Збірка

error-joint-unsatisfied = Спряження не виконано — нев’язка { $residual } мм

## Вирази

error-expr-unknown-char = Невідомий символ «{ $what }»
error-expr-unknown-fn = Невідома функція «{ $what }»
error-expr-unknown-name = невідома назва: { $what } — такого параметра немає
error-expr-needs-one-arg = { $what }() приймає один аргумент
error-expr-needs-two-args = { $what }() приймає два аргументи
error-expr-expected-paren = Очікувалася «)»
error-expr-expected-paren-after-args = Очікувалася «)» після аргументів
error-expr-unexpected-token = Неочікуваний елемент { $what }
error-expr-unexpected-end = вираз закінчується зарано: очікувалося число або назва
error-expr-trailing-input = Зайве закінчення за «{ $what }»
error-expr-not-a-number = Результат — не число (ділення на нуль?)
error-expr-cycle = формула повертається до { $what } — безпосередньо або через інші параметри: приберіть назву з формули або розірвіть ланцюжок

## Повідомлення самого ядра — передається без перекладу: це діагностика, а не проза.

error-kernel-message = Ядро: { $message }

# -- МІСТОК ДО ГЕОМЕТРИЧНОГО ЯДРА (OCCT) --
cad-no-faces-picked = не вибрано жодної грані
cad-faces-not-in-body = вибраних граней немає в цьому тілі (посилання застаріло)
cad-neighbours-not-extendable = сусідні поверхні не подовжуються — прибирається цілий елемент (отвір, бобишка)
cad-file-not-found = Файл не знайдено: { $v }
cad-step-no-shapes = STEP: тіла не вдалося прочитати
cad-step-nothing-to-export = STEP: немає тіл для експорту
cad-step-write-failed = STEP: запис не вдався (код { $v })
cad-step-read-failed = STEP: геометрію не вдалося прочитати або передати
cad-iges-no-shapes = IGES: тіла не вдалося прочитати
cad-iges-read-failed = IGES: геометрію не вдалося прочитати або передати
cad-iges-empty-tessellation = IGES: у файлі немає поверхні, яку можна показати
cad-iges-nothing-to-export = IGES: нічого записувати
cad-iges-write-failed = IGES: запис не вдався (код { $v })
io-iges-read-failed = IGES: файл не вдається прочитати ({ $v })
io-iges-not-iges = Це не IGES: у файлі немає жодного з розділів, з яких складається IGES
io-iges-no-curves = IGES: у файлі немає ні поверхонь, ні кривих, які можна показати
io-obj-read-failed = OBJ: файл не вдається прочитати ({ $v })
io-obj-bad-line = OBJ: рядок { $v } не вдалося прочитати
io-obj-bad-index = OBJ: грань у рядку { $v } посилається на вершину, якої немає
io-obj-no-faces = OBJ: у файлі немає жодної грані
io-obj-no-triangles = OBJ: нічого записувати
io-obj-write-failed = OBJ: запис не вдався ({ $v })
io-ply-read-failed = PLY: файл не вдається прочитати ({ $v })
io-ply-not-ply = Це не PLY: файл не починається із заголовка PLY
io-ply-bad-header = PLY: заголовок не вдалося прочитати
io-ply-truncated = PLY: файл закінчується раніше, ніж каже його заголовок
io-ply-bad-index = PLY: грань посилається на вершину, якої немає
io-ply-no-faces = PLY: у файлі немає жодної грані
io-ply-no-triangles = PLY: нічого записувати
io-ply-write-failed = PLY: запис не вдався ({ $v })
io-gltf-read-failed = glTF: файл не вдається прочитати ({ $v })
io-gltf-not-gltf = Це не glTF: у файлі немає опису сцени
io-gltf-truncated = glTF: двійковий файл обірвано
io-gltf-bad-node = glTF: вузол сцени посилається на те, чого немає
io-gltf-no-positions = glTF: сітка не несе положень вершин
io-gltf-bad-index = glTF: трикутник посилається на вершину, якої немає
io-gltf-bad-accessor = glTF: дані сітки описано хибно
io-gltf-no-buffer = glTF: у файлі бракує двійкової частини, яку він називає
io-gltf-bad-buffer = glTF: вбудовані дані не вдається розкодувати
io-gltf-missing-buffer = glTF: його даних «{ $v }» немає поруч із файлом
io-gltf-no-meshes = glTF: у сцені немає жодної сітки
io-gltf-no-triangles = glTF: нічого записувати
io-gltf-write-failed = glTF: запис не вдався ({ $v })
io-3mf-read-failed = 3MF: файл не вдається прочитати ({ $v })
io-3mf-not-3mf = Це не 3MF: файл не є архівом-пакетом
io-3mf-no-model = 3MF: у пакеті немає моделі
io-3mf-bad-model = 3MF: модель описано хибно
io-3mf-unknown-unit = 3MF: невідома одиниця «{ $v }»
io-3mf-bad-transform = 3MF: перетворення записано хибно
io-3mf-no-object = 3MF: модель посилається на об’єкт { $v }, якого немає
io-3mf-bad-index = 3MF: трикутник посилається на вершину, якої немає
io-3mf-no-meshes = 3MF: у моделі немає жодної сітки
io-3mf-no-triangles = 3MF: нічого записувати
io-3mf-write-failed = 3MF: запис не вдався ({ $v })
io-amf-read-failed = AMF: файл не вдається прочитати ({ $v })
io-amf-not-amf = Це не AMF: у файлі немає розмітки AMF
io-amf-unknown-unit = AMF: невідома одиниця «{ $v }»
io-amf-bad-vertex = AMF: вершину записано хибно
io-amf-bad-triangle = AMF: трикутник записано хибно
io-amf-bad-index = AMF: трикутник посилається на вершину, якої немає
io-amf-bad-constellation = AMF: сузір’я описано хибно
io-amf-no-object = AMF: сузір’я посилається на об’єкт { $v }, якого немає
io-amf-no-meshes = AMF: у файлі немає жодної сітки
io-amf-no-triangles = AMF: нічого записувати
io-amf-write-failed = AMF: запис не вдався ({ $v })
cad-step-empty-tessellation = STEP: порожня тесселяція (немає тіл/граней?)
cad-extrude-needs-3-points = профілю видавлювання потрібно >=3 точки
cad-extrude-failed = OCCT: профіль не вдалося видавити (самоперетин?)
cad-extrude-empty = видавлювання дало порожнє тіло
cad-revolve-needs-3-points = профілю обертання потрібно >=3 точки
cad-revolve-failed = OCCT: обертання не вдалося (чи не перетинає профіль вісь?)
cad-revolve-empty = обертання дало порожнє тіло
cad-boolean-needs-3-points = обом профілям потрібно >=3 точки
cad-boolean-failed = OCCT: булева операція не вдалася
cad-boolean-empty = булева операція дала порожнє тіло

# -- ФАЙЛОВИЙ ШАР: коди приходять з qymcad-io, аргумент — шлях і текст ОС --
io-file-create = не вдалося створити { $v }
io-file-replace = не вдалося замінити { $v }
io-file-read = не вдалося прочитати { $v }
io-not-a-qpart = це не .qpart (не zip-контейнер)
io-not-a-qcad = це не .qcad (не zip-контейнер): старий формат не підтримується
io-refuse-empty-over-full = відмовлено: порожній документ поверх непорожнього файлу (вузлів: { $v }) — збережіть його як новий файл
io-stl-read-failed = STL: файл не вдається прочитати ({ $v })
io-stl-truncated = STL: файл закінчується раніше за трикутники, які налічує його заголовок
io-stl-bad-facet = STL: грань-трикутник не вдалося прочитати
io-stl-no-faces = STL: у файлі немає жодного трикутника
io-stl-no-triangles = STL: немає трикутників для експорту
io-stl-too-many-triangles = STL: забагато трикутників
io-stl-write-failed = STL: запис не вдався: { $v }

io-svg-empty-sketch = SVG: ескіз порожній
io-svg-write-failed = SVG: запис не вдався: { $v }
io-dxf-empty-sketch = DXF: ескіз порожній
io-dxf-write-failed = DXF: запис не вдався: { $v }
verify-axis-out-of-table = хід виходить за стіл — { $v }
post-not-implemented = постпроцесор ще не реалізовано
error-edges-not-found = Жодного з { $asked } названих ребер у тілі не лишилося. Їхні назви прийшли від операції вище в стрічці, і вона змінилася — виберіть ребра заново.
error-op-failed-patch = Поверхня не натягується на ці ребра
error-shell-thickness-over-round = Стінка { $t } мм товща за найменше заокруглення на тілі ({ $r } мм): зміщення з’їдає його цілком, і оболонку не можна побудувати. Візьміть стінку тоншу за { $r } мм або збільште заокруглення
error-operation-split-body = Операція розділила деталь на тіла: { $n } — деталь тримає рівно одне тіло. Зменште значення або застосуйте операцію до іншої грані
error-mirror-of-hollow-body = Віддзеркалення порожнистої деталі відносно її власної грані поки що поза можливостями ядра: під час з’єднання половин лишаються зайві оболонки. Віддзеркальте деталь до оболонки або виберіть іншу площину
error-shell-of-multi-shell-body = Ядро не може зробити оболонку з тіла з { $n } оболонок: воно вже порожнисте або зібране з копій (масив, дзеркало). Робіть оболонку раніше — до масиву, дзеркала чи другої оболонки
error-shell-not-built-here = Оболонку не вдалося побудувати на цьому тілі: зміщення граней не вдається всередині ядра. Спробуйте іншу товщину стінки або зробіть оболонку раніше в історії, поки тіло простіше
error-cut-removed-nothing = Виріз нічого не прибрав: інструмент не перетинає деталь. Перевірте, де стоїть інструмент і наскільки глибоко йде виріз
error-stitch-nothing-joined = Зшивати нічого: вибрані поверхні не мають спільних ребер — вони не торкаються. Після заокруглень сусідні грані розділені округлою смугою; виберіть поверхні, що справді сходяться
