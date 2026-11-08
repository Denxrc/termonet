<div align="center">

```text
▀█▀ █▀▀▀ █▀▀█ █▀▄▀█ █▀▀█ █▄ █ █▀▀▀ ▀█▀
 █  █▀▀  █▄▄▀ █ █ █ █  █ █ ▀█ █▀▀   █ 
 █  █▄▄▄ █  █ █   █ █▄▄█ █  █ █▄▄▄  █ 
```

# Termonet

**Быстрый и динамический генератор системных тем и палитр Material You для Linux**

[![Rust Edition 2024](https://img.shields.io/badge/rust-edition_2024-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/версия-1.0.0-blue.svg)](Cargo.toml)
[![License: MIT](https://img.shields.io/badge/лицензия-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/платформа-Linux%20(Wayland%20%2F%20X11)-lightgrey.svg)]()

[English](README.md) | [Русский](README.ru.md)

</div>

---

## 🌟 О проекте

**Termonet** — легковесный, безопасный генератор динамических тем оформления и цветовых палитр для графических окружений Linux (Wayland и X11), написанный на **Rust** (edition 2024).

Программа извлекает ключевые цвета из обоев рабочего стола и математически гармонизирует их по алгоритму **Google Material You (Material 3)**, создавая роли цветов и 16-цветную ANSI-палитру терминала. Termonet оснащен удобным терминальным интерфейсом (TUI) на базе `ratatui` с интерактивным мастером настройки, вводом путей к обоям прямо в интерфейсе и безопасным управлением конфигурацией.

---

## ✨ Возможности

- 🎨 **Настоящий Material You (Material 3)**: На базе крейта `material-colors` с использованием алгоритма `Variant::Content`, который сохраняет естественные цвета и приглушенные атмосферные полутона без искусственного завышения насыщенности.
- ⚡ **Мгновенная обработка изображений**: Предварительное масштабирование до 128×128 в памяти через крейт `image`, обеспечивающее расчет палитры всего за ~40 мс для изображений любого разрешения, включая 8K и анимированные GIF-файлы (извлечение первого кадра).
- 🖥️ **Адаптивный TUI с фирменным 5px пиксельным шрифтом**:
  - Пиксельный логотип высотой 5 строк через полублоки Unicode (`█`, `▀`, `▄`) с поддержкой кириллицы и латиницы.
  - Автоматическое переключение на компактный заголовок при малом размере терминала.
- 🧙 **Интерактивный мастер первого запуска**:
  - Полная локализация на **русский** и **английский** языки.
  - Выбор темы оформления TUI (Темная / Светлая / Системная), графического сервера (Wayland / X11) и бэкенда обоев.
  - Вызов в любой момент по клавише `s` или `F2`.
- 🖼️ **Смена обоев «на лету» в интерфейсе**:
  - Нажатие `w` на шаге палитры открывает модальное окно для ввода пути.
  - Автоматическое раскрытие тильды `~` в домашнюю директорию (`~/Pictures/wall.png` ➔ `/home/user/Pictures/wall.png`).
  - Мгновенный пересчет цветов и сохранение в конфиг.
- 📝 **Понятный и комментированный конфиг**:
  - Файл `~/.config/termonet/config.toml` снабжен подробными пояснениями к каждой опции и списком поддерживаемых значений.
- 🛡️ **Безопасность и работа без root-прав**:
  - Работает строго с правами обычного пользователя.
  - Все операции до подтверждения выполняются в оперативной памяти.

---

## 🧭 Навигация и горячие клавиши

Интерфейс разделен на 4 логических шага:

| Шаг | Экран | Описание |
| :---: | :--- | :--- |
| **1** | **Палитра** | Просмотр ролей Material You и 16 цветов ANSI; нажатие `w` открывает ввод пути к обоям |
| **2** | **Программы** | Выбор поддерживаемых приложений (Kitty, Waybar, Rofi, Hyprland, Alacritty, Dunst, Sway) |
| **3** | **Хуки** | Просмотр и переключение команд перезапуска сервисов (напр. `hyprctl reload`, `killall -SIGUSR2 waybar`) |
| **4** | **Применение** | Сводка обновляемых файлов и запускаемых хуков перед записью |

### Управление

| Клавиша | Действие |
| :--- | :--- |
| `Tab` / `Shift+Tab` | Переход к следующему / предыдущему шагу |
| `1`, `2`, `3`, `4` | Прямой переход к соответствующему шагу |
| `↑` / `↓` (`k` / `j`) | Навигация по спискам программ и хуков |
| `Пробел` | Включение / отключение выбранной программы или хука |
| `w` | Открыть ввод пути к обоям (на шаге 1) |
| `s` / `F2` | Открыть мастер настроек |
| `Esc` | Закрыть модальное окно ввода / отмена настроек |
| `q` | Выход из Termonet |

---

## 🚀 Установка и сборка

### Требования

- Linux (Wayland или X11)
- Компилятор Rust (1.85+ с поддержкой редакции 2024):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

### Сборка из исходников

```bash
git clone https://github.com/Denxrc/termonet.git
cd termonet
cargo build --release
```

Готовый бинарный файл находится в `target/release/termonet`. Для установки в систему:

```bash
install -Dm755 target/release/termonet ~/.local/bin/termonet
```

---

## 💻 Использование

```bash
# Запуск TUI (автоматически подхватит обои из конфига или откроет мастер при первом запуске)
termonet

# Запуск с указанием конкретного изображения обоев
termonet -w ~/Pictures/Wallpapers/forest.png

# Запуск мастера начальной настройки
termonet --init
```

---

## ⚙️ Конфигурация

Файл настроек располагается по пути `~/.config/termonet/config.toml` и содержит подробные комментарии:

```toml
# ==============================================================================
# Termonet Configuration
# Путь: ~/.config/termonet/config.toml
# ==============================================================================

# Язык интерфейса ("ru" | "en")
language = "ru"

# Цветовая тема TUI ("dark" | "light" | "system")
tui_theme = "dark"

# Режим шрифта заголовка ("pixel" | "standard")
font_mode = "pixel"

# Графический сервер ("wayland" | "x11")
display_server = "wayland"

# Утилита обоев
# Варианты: "auto", "swww", "hyprpaper", "awww", "wpaperd", "swaybg", "gnome", "kde", "feh", "nitrogen", "manual"
wallpaper_backend = "auto"

# Главный монитор (раскомментируйте для указания)
# primary_monitor = "eDP-1"

# Ручной путь к обоям (раскомментируйте для указания)
# manual_wallpaper_path = "~/Pictures/wallpapers/forest.png"

# Программы для генерации цветовых схем
enabled_programs = [
    "kitty",
    "waybar",
    "rofi",
    "hyprland"
]
```

---

## 🎨 Создание собственных правил и шаблонов тем

Termonet спроектирован полностью модульным: вы можете добавить динамическую поддержку **любой** программы (терминала, бара, лаунчера, редактора кода), просто создав два файла в каталоге `~/.config/termonet/`.

### 1. Архитектура темы

Интеграция каждой программы описывается двумя компонентами:
1. **Файл правил** (`~/.config/termonet/rules/<программа>.toml`) — метаданные, пути к файлам и команда перезапуска.
2. **Файл шаблона** (`~/.config/termonet/templates/<программа>.tmpl`) — разметка конфига с синтаксисом MiniJinja.

### 2. Спецификация файла правил (`rules/<name>.toml`)

```toml
# Версия схемы правила (для автоматической миграции дефолтов)
version = 2

# Имя программы (отображается в TUI и мастере)
name = "Foot"

# Имя файла шаблона в ~/.config/termonet/templates/
template = "foot.tmpl"

# Куда сохранять сгенерированный файл цветов
output_path = "~/.config/foot/colors-termonet.ini"

# (Опционально) Главный конфигурационный файл программы для вставки импорта
target_config = "~/.config/foot/foot.ini"

# (Опционально) Строка импорта, которая автоматически вставится в target_config
import_line = "include = ~/.config/foot/colors-termonet.ini"

# Позиция вставки ("top" - в начало, "bottom" - в конец)
import_position = "bottom"

# (Опционально) Стиль комментариев маркера ("hash" | "blockcss" | "slash" | "doubledash")
# Если не указан, определяется автоматически по расширению файла target_config
comment_style = "hash"

# (Опционально) Команда для мгновенного применения темы без перезапуска программы
reload_cmd = ""
```

### 3. Переменные и фильтры в шаблонах (`templates/<name>.tmpl`)

В шаблонах доступны все роли **Google Material You** (для светлой и темной тем), а также гармонизированная 16-цветная ANSI-палитра терминала.

#### Доступные переменные

- **Роли Material You (`dark.*` и `light.*`)**:
  - `dark.primary`, `dark.on_primary`, `dark.primary_container`, `dark.on_primary_container`
  - `dark.secondary`, `dark.on_secondary`, `dark.secondary_container`, `dark.on_secondary_container`
  - `dark.tertiary`, `dark.on_tertiary`
  - `dark.surface`, `dark.on_surface`, `dark.surface_variant`, `dark.on_surface_variant`
  - `dark.background`, `dark.on_background`, `dark.outline`
- **Терминальная ANSI-палитра (`terminal.*`)**:
  - `terminal.color0` ... `terminal.color15`
  - `terminal.background`, `terminal.foreground`, `terminal.cursor`
- **Ключевой цвет обоев**: `seed_color`

#### Форматы вывода цветов

Каждый цвет предоставляет готовые поля:
- `.hex` — цвет с решеткой: `#adc6ff`
- `.strip` — цвет без решетки: `adc6ff`
- `.rgb` — каналы через запятую: `173, 198, 255`
- `.r`, `.g`, `.b` — отдельные 8-битные каналы (0..255)

#### Встроенные фильтры MiniJinja

- `{{ dark.primary | hex }}` ➔ гарантированный `#adc6ff`
- `{{ dark.primary | strip }}` (или `stripped`) ➔ `adc6ff`
- `{{ dark.primary | rgb }}` ➔ `173, 198, 255`
- `{{ dark.primary | hypr_rgb }}` ➔ `rgb(adc6ff)`
- `{{ dark.primary | hypr_rgba("ee") }}` ➔ `rgba(adc6ffee)`

### 4. Практический пример: Добавление терминала Foot

1. Создайте `~/.config/termonet/rules/foot.toml`:
   ```toml
   version = 2
   name = "Foot"
   template = "foot.tmpl"
   output_path = "~/.config/foot/colors-termonet.ini"
   target_config = "~/.config/foot/foot.ini"
   import_line = "include = ~/.config/foot/colors-termonet.ini"
   import_position = "bottom"
   ```

2. Создайте `~/.config/termonet/templates/foot.tmpl`:
   ```ini
   # Termonet Foot Terminal Theme
   # termonet-version: 2

   [colors]
   background = {{ terminal.background.strip }}
   foreground = {{ terminal.foreground.strip }}
   regular0 = {{ terminal.color0.strip }}
   regular1 = {{ terminal.color1.strip }}
   regular2 = {{ terminal.color2.strip }}
   regular3 = {{ terminal.color3.strip }}
   regular4 = {{ terminal.color4.strip }}
   regular5 = {{ terminal.color5.strip }}
   regular6 = {{ terminal.color6.strip }}
   regular7 = {{ terminal.color7.strip }}
   bright0 = {{ terminal.color8.strip }}
   bright1 = {{ terminal.color9.strip }}
   bright2 = {{ terminal.color10.strip }}
   bright3 = {{ terminal.color11.strip }}
   bright4 = {{ terminal.color12.strip }}
   bright5 = {{ terminal.color13.strip }}
   bright6 = {{ terminal.color14.strip }}
   bright7 = {{ terminal.color15.strip }}
   ```

3. Запустите `termonet` — программа автоматически появится в списке на шаге 2!

### 5. Безопасность и автоматические бэкапы

Termonet заботится о сохранности ваших исходных конфигураций:
- **Эталонный бэкап (`.original`)**: создается строго один раз при первом внедрении импорта и **никогда** не перезаписывается последующими темами.
- **Страховочный бэкап (`.bak`)**: создается перед каждым изменением файла.
- **Умные маркеры**: инъекция оформляется блоком комментариев:
  ```ini
  # >>> termonet colors >>>
  include colors-termonet.conf
  # <<< termonet colors <<<
  ```
  Если в конфиге присутствовал старый импорт Pywal, Termonet аккуратно заменяет его на месте, не плодя дубликатов.
- **Принудительное обновление дефолтов**: при обновлении Termonet встроенные шаблоны можно обновить командой:
  ```bash
  termonet --update-rules
  ```

---

## 🗺️ План разработки (Roadmap)

- [x] **v0.x (Прототип и база)**:
  - [x] Адаптивный TUI на `ratatui` с 5px пиксельным шрифтом
  - [x] Генерация палитры Material You через `material-colors` (`Variant::Content`)
  - [x] Мастер первого запуска и комментированный TOML-конфиг
  - [x] Двуязычный интерфейс (русский и английский)
  - [x] Ручной ввод пути к обоям в TUI и автоподстановка
- [ ] **v1.x (Полноценный релиз для Wayland)**:
  - [x] Модульный шаблонизатор `minijinja` с фильтрами и переменными TrueColor / ANSI
  - [x] Встроенные готовые правила и шаблоны (Kitty, Waybar, Rofi, Hyprland Lua)
  - [x] Безопасная инъекция маркеров, замена legacy-импортов (Pywal) и система вечных `.original` бэкапов
  - [x] Двухуровневое версионирование и автомиграция правил/шаблонов (`--update-rules`)
  - [ ] Автоматическое определение обоев в Wayland (`swww`, `hyprpaper`, `awww`, `wpaperd`, `swaybg`, `GNOME`, `KDE`)
  - [ ] Мультимониторный фильтр и выбор главного экрана
  - [ ] Флаг автоприменения без графики (`--apply`) для скриптов и systemd
- [ ] **v2.x (Расширение экосистемы)**:
  - [ ] Автоопределение в X11 (`feh`, `nitrogen`)
  - [ ] Переопределение параметров темы через CLI
  - [ ] Расширенная база шаблонов приложений

---

## 📄 Лицензия

Проект распространяется под открытой лицензией [MIT License](LICENSE).
