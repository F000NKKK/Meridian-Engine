#!/usr/bin/env bash
# release.sh — bump, publish, и обновить ссылки в воркспейсе.
# Использование: ./release.sh <crate-name> [--patch|--minor|--major] [--dry-run] [--no-publish] [--no-cascade] [--no-check-ver]
#                ./release.sh --publish-all [--patch|--minor|--major] [--dry-run] [--no-publish] [--no-check-ver]
#
# При --minor/--major все зависимые крейты воркспейса тоже бампятся (минор) и
# публикуются следом в порядке зависимостей — иначе на crates.io остаются
# версии, собранные против старого минора (в 0.x каждый минор несовместим),
# и verify следующего крейта падает на «two different versions of crate».
#
# --publish-all заменяет <crate-name>: план строится из всех крейтов
# воркспейса (топологически), а не из каскада зависимых одного корня.
#
# Бамп теперь всегда опциональный — без --patch/--minor/--major (или явно
# с --no-bump) ничего не бампается: для каждого крейта из плана проверяем
# crates.io, и публикуем только то, что ещё не опубликовано.
#
# Проверка crates.io перед бампом (чтобы не перескочить версию, которая была
# сбампана в git, но так и не опубликована) включается только для «круглых»
# версий относительно запрошенного бампа:
#   --patch  — не круглый бамп, проверка не нужна, бампаем всегда
#   --minor  — круглый, если patch == 0    (например 1.9.0 --minor: patch=0 → проверка)
#   --major  — круглый, если minor==0 и patch==0 (например 2.0.0 --major → проверка;
#              1.9.0 --major: minor=9≠0 → не круглый, бампаем сразу в 2.0.0)
# Если версия ещё не опубликована — бамп пропускается, публикуется текущая
# версия как есть. --no-check-ver отключает эту проверку полностью (старое
# поведение: бампаем/публикуем вслепую, без обращения к crates.io).
#
# Примеры:
#   ./release.sh meridian-gac-core --minor        # каскад: gac-core → ecs-core → ... → engine-core
#   ./release.sh meridian-engine-core --patch     # patch: ссылки совместимы, каскада нет
#   ./release.sh meridian-gac-core                # без бампа: публикует v текущую, если ещё не на crates.io
#   ./release.sh --publish-all                    # весь воркспейс как есть, публикует неопубликованное
#   ./release.sh --publish-all --patch            # patch-бамп + публикация всех крейтов
#
# Перед бампом/публикацией (если не --skip-checks) прогоняется preflight:
# cargo fmt --check, cargo clippy -D warnings, cargo test, cargo doc,
# scripts/check_dependency_rules.py (блокирует релиз при нарушении
# docs/dependency-rules.md — не warning, а die) и, если установлен
# cargo-semver-checks, semver-сверка каждого крейта из плана против его
# опубликованной версии (soft — предупреждает, не блокирует, т.к. умеет
# ложно сработать на скаффолде без реального API).
#
# Какой бамп когда (человеческая дисциплина, скриптом не проверяется):
#   --patch  — багфиксы, без изменения публичного API
#   --minor  — завершённый функциональный этап (numeric-core, gac-core,
#              ecs-core, ...) — то, ради чего вообще стоит делать релиз
#   --major  — изменение публичной архитектуры (breaking change)
# Не релизь просто чтобы был релиз — 19 релизов сейчас отражают 19 реально
# завершённых этапов, не счётчик активности.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WS="$SCRIPT_DIR"
CRATE_PREFIX="meridian-"
CRATES_IO_USER_AGENT="release.sh (Meridian-Engine; https://github.com/F000NKKK/Meridian-Engine)"

# ── Цвета ─────────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

info()    { echo -e "${CYAN}[release]${RESET} $*"; }
ok()      { echo -e "${GREEN}[ok]${RESET} $*"; }
warn()    { echo -e "${YELLOW}[warn]${RESET} $*"; }
die()     { echo -e "${RED}[error]${RESET} $*" >&2; exit 1; }
dryrun()  { echo -e "${YELLOW}[dry-run]${RESET} $*"; }

# ── Аргументы ─────────────────────────────────────────────────────────────────
usage() {
    echo -e "${BOLD}Использование:${RESET} $0 <crate-name> [--patch|--minor|--major] [--dry-run] [--no-publish] [--no-cascade] [--no-check-ver] [--no-gh-release]"
    echo -e "           $0 --publish-all [--patch|--minor|--major] [--dry-run] [--no-publish] [--no-check-ver] [--no-gh-release]"
    echo ""
    echo "  --patch          0.1.3 → 0.1.4  (ссылки не меняются — semver совместимость)"
    echo "  --minor          0.1.3 → 0.2.0  (обновит ссылки и каскадно сбампит зависимые крейты)"
    echo "  --major          0.1.3 → 1.0.0  (обновит ссылки и каскадно сбампит зависимые крейты)"
    echo "  (без бампа)      не менять version — опубликовать план как есть, что ещё не на crates.io"
    echo "  --no-bump        то же самое явно (алиас «без бампа»)"
    echo "  --dry-run        только показать, что изменится"
    echo "  --no-publish     сбампить и обновить ссылки без публикации"
    echo "  --no-cascade     не трогать зависимые крейты"
    echo "  --no-check-ver   не проверять crates.io — бампать/публиковать вслепую"
    echo "  --no-gh-release  не создавать git-теги и GitHub-релизы после публикации"
    echo "  --skip-checks    не гонять preflight (fmt/clippy/test/doc/check-deps/semver-checks)"
    echo "  --publish-all    заменяет <crate-name>: план — все крейты воркспейса, топологически"
    exit 1
}

CRATE=""
BUMP=""
DRY_RUN=false
NO_PUBLISH=false
PUBLISH_ALL=false
NO_BUMP=false
NO_CHECK_VER=false
NO_GH_RELEASE=false
SKIP_CHECKS=false
CASCADE=true

for arg in "$@"; do
    case "$arg" in
        --patch|--minor|--major) BUMP="$arg" ;;
        --dry-run)      DRY_RUN=true ;;
        --no-publish)   NO_PUBLISH=true ;;
        --publish-all)  PUBLISH_ALL=true ;;
        --no-bump)      NO_BUMP=true ;;
        --no-check-ver) NO_CHECK_VER=true ;;
        --no-cascade)   CASCADE=false ;;
        --no-gh-release) NO_GH_RELEASE=true ;;
        --skip-checks)  SKIP_CHECKS=true ;;
        --*) die "Неизвестный флаг: $arg"; ;;
        *)
            if [[ -z "$CRATE" ]]; then CRATE="$arg"
            else die "Лишний аргумент: $arg"; fi
            ;;
    esac
done

if $PUBLISH_ALL; then
    [[ -n "$CRATE" ]] && die "--publish-all нельзя сочетать с именем крейта ($CRATE)"
else
    [[ -z "$CRATE" ]] && { echo "Не указано имя крейта (или используйте --publish-all)."; usage; }
fi
# Бамп всегда опционален: явный --no-bump или просто отсутствие
# --patch/--minor/--major — синонимы «не бампать».
$NO_BUMP && BUMP=""
# patch-бамп совместим по ссылкам — каскад не нужен
[[ "$BUMP" == "--patch" ]] && CASCADE=false

crate_toml() { echo "$WS/crates/$1/Cargo.toml"; }

crate_version() {
    grep -m1 '^version *= *"' "$(crate_toml "$1")" | sed 's/.*"\([^"]*\)".*/\1/'
}

$PUBLISH_ALL || { [[ -f "$(crate_toml "$CRATE")" ]] || die "Крейт не найден: $(crate_toml "$CRATE")"; }

# ── crates.io: опубликована ли данная версия? ─────────────────────────────────
crate_is_published() {
    local crate="$1" ver="$2" code
    code="$(curl -s -o /dev/null -w '%{http_code}' \
        -A "$CRATES_IO_USER_AGENT" \
        "https://crates.io/api/v1/crates/${crate}/${ver}" 2>/dev/null)" || code="000"
    [[ "$code" == "200" ]]
}

# ── Preflight ────────────────────────────────────────────────────────────────
# Гоняется один раз, до любых правок Cargo.toml. Только dependency-rules —
# жёсткий блок (die); fmt/clippy/test/doc — тоже блок, т.к. это то же самое,
# что уже проверяет CI, и незачем публиковать то, что там не проходит;
# semver-checks — мягкий (warn), т.к. на крейте, у которого ещё не было
# осмысленного публичного API, он может ложно шуметь.
# $1: список крейтов из плана (для semver-checks), через пробел.
run_preflight() {
    local plan="$1"

    if $SKIP_CHECKS; then
        warn "--skip-checks: пропускаю preflight (fmt/clippy/test/doc/check-deps/semver-checks)"
        return 0
    fi

    info "Preflight: dependency-rules (scripts/check_dependency_rules.py) ..."
    python3 "$WS/scripts/check_dependency_rules.py" \
        || die "dependency-rules нарушены — релиз запрещён, см. docs/dependency-rules.md"

    info "Preflight: cargo fmt --check ..."
    (cd "$WS" && cargo fmt --check) \
        || die "cargo fmt --check провалился — прогони 'cargo fmt' и закоммить"

    info "Preflight: cargo clippy --workspace --all-targets ..."
    cargo clippy --manifest-path "$WS/Cargo.toml" --workspace --all-targets --quiet -- -D warnings \
        || die "cargo clippy нашёл проблемы"

    info "Preflight: cargo test --workspace ..."
    cargo test --manifest-path "$WS/Cargo.toml" --workspace --quiet \
        || die "cargo test провалился"

    info "Preflight: cargo doc --no-deps --workspace ..."
    cargo doc --manifest-path "$WS/Cargo.toml" --no-deps --workspace --quiet \
        || die "cargo doc провалился"

    if command -v cargo-semver-checks >/dev/null 2>&1; then
        info "Preflight: cargo semver-checks (по крейтам из плана) ..."
        for c in $plan; do
            if ! cargo semver-checks check-release -p "$c" --manifest-path "$WS/Cargo.toml" 2>&1 | tail -25; then
                warn "$c: semver-checks нашёл потенциально breaking изменения — если это не --major, проверь вручную"
            fi
        done
    else
        warn "cargo-semver-checks не установлен — пропускаю (cargo install cargo-semver-checks)"
    fi

    ok "Preflight пройден."
}

# ── Публикация с ретраями ─────────────────────────────────────────────────────
# Без фиксированного ожидания индексации: cargo сам ждёт появления крейта в
# индексе после аплоада, а если следующий publish всё же не резолвит только
# что изданную зависимость — ретраим: 1 попытка + 3 ретрая раз в 5 сек.
publish_with_retry() {
    local crate="$1" ver="$2"
    local attempt=0 max=4 out rc
    while true; do
        attempt=$((attempt + 1))
        info "Публикую $crate v$ver на crates.io (попытка $attempt/$max)..."
        set +e
        out=$(cargo publish -p "$crate" --manifest-path "$WS/Cargo.toml" 2>&1); rc=$?
        set -e
        if [[ $rc -eq 0 ]]; then
            ok "Опубликовано: $crate v$ver"
            return 0
        fi
        if echo "$out" | grep -qiE "already uploaded|already exists on crates.io"; then
            warn "$crate v$ver уже на crates.io — пропускаю"
            return 0
        fi
        if [[ $attempt -ge $max ]]; then
            echo "$out" | tail -40
            die "Публикация $crate v$ver не удалась после $max попыток"
        fi
        echo "$out" | tail -5
        warn "Не удалось (rc=$rc) — ретрай через 5 сек..."
        sleep 5
    done
}

# ── Функция замены версии-ссылки в файле ──────────────────────────────────────
update_ref() {
    local dep="$1" file="$2" old="$3" new="$4"
    # Простая форма:  meridian-foo = "0.1"  /  "0.1.2"
    sed -i -E "s|(${dep}[[:space:]]*=[[:space:]]*\")${old}([.\"])|\1${new}\2|g" "$file"
    # Сложная форма:  meridian-foo = { version = "0.1", ... }
    sed -i -E "s|(${dep}[[:space:]]*=[[:space:]]*\{[^}]*version[[:space:]]*=[[:space:]]*\")${old}([.\"])|\1${new}\2|g" "$file"
}

# ── Круглая ли версия относительно запрошенного бампа? ────────────────────────
# Круглая версия — та, где не проверив crates.io нельзя быть уверенным, что
# она реально опубликована (а не просто сбампана в git прошлым запуском).
is_round_for_bump() {
    local bump="$1" min="$2" pat="$3"
    case "$bump" in
        --patch) return 1 ;;
        --minor) [[ "$pat" == "0" ]] ;;
        --major) [[ "$min" == "0" && "$pat" == "0" ]] ;;
        *)       return 1 ;;
    esac
}

# ── Решение по одному крейту: новая версия + публиковать или пропустить ───────
# resolve_crate_action <crate> <--patch|--minor|--major|(пусто)>
# Печатает на stdout ровно одну строку: "<version> <publish|skip>".
# Статусы — в stderr.
resolve_crate_action() {
    local crate="$1" bump="$2"
    local toml; toml="$(crate_toml "$crate")"
    local current; current="$(crate_version "$crate")"
    [[ -n "$current" ]] || die "Не удалось прочитать version из $toml"

    if [[ -z "$bump" ]]; then
        if $NO_CHECK_VER; then
            echo "$current publish"
            return 0
        fi
        info "Проверяю crates.io: $crate v$current ..." >&2
        if crate_is_published "$crate" "$current"; then
            warn "  $crate v$current уже опубликован — пропускаю" >&2
            echo "$current skip"
        else
            ok "  $crate v$current ещё не опубликован — публикую как есть" >&2
            echo "$current publish"
        fi
        return 0
    fi

    local maj min pat
    IFS='.' read -r maj min pat <<< "$current"

    if ! $NO_CHECK_VER && is_round_for_bump "$bump" "$min" "$pat"; then
        info "Проверяю crates.io: $crate v$current ($bump — круглая версия) ..." >&2
        if crate_is_published "$crate" "$current"; then
            ok "  $crate v$current опубликован — бампаю" >&2
        else
            warn "  $crate v$current ещё не опубликован — публикую как есть, без бампа" >&2
            echo "$current publish"
            return 0
        fi
    fi

    case "$bump" in
        --major) maj=$((maj + 1)); min=0; pat=0 ;;
        --minor) min=$((min + 1)); pat=0 ;;
        --patch) pat=$((pat + 1)) ;;
    esac
    local new_version="$maj.$min.$pat"
    local old_short new_short
    old_short="$(echo "$current" | cut -d. -f1-2)"
    new_short="$maj.$min"

    if $DRY_RUN; then
        dryrun "$crate: $current → $new_version" >&2
    else
        awk -v old="$current" -v new="$new_version" '
            /^\[/{in_pkg=0}
            /^\[package\]/{in_pkg=1}
            in_pkg && /^version *=/ && !done {
                sub(old, new); done=1
            }
            {print}
        ' "$toml" > "$toml.tmp" && mv "$toml.tmp" "$toml"
        ok "$crate: $current → $new_version" >&2
    fi

    # Единственное место со ссылкой на версию крейта — [workspace.dependencies]
    # в корневом Cargo.toml (member-крейты используют `workspace = true` и
    # ничего не хранят локально). Обновляем только при смене минора/мажора.
    if [[ "$old_short" != "$new_short" ]]; then
        local root_toml="$WS/Cargo.toml"
        if grep -qE "${crate}[[:space:]]*=.*\"${old_short}\"" "$root_toml" 2>/dev/null; then
            if $DRY_RUN; then
                dryrun "  Cargo.toml [workspace.dependencies] : $crate $old_short → $new_short" >&2
            else
                update_ref "$crate" "$root_toml" "$old_short" "$new_short"
                ok "  ссылка: Cargo.toml [workspace.dependencies]" >&2
            fi
        fi
    fi

    echo "$new_version publish"
}

# ── Порядок публикации ─────────────────────────────────────────────────────────
# Печатает "<crate> <crate> ..." топологически (зависимости раньше зависящих).
# root="" → весь воркспейс (--publish-all); root=<crate> → сам корень + все
# транзитивные зависящие от него (обычный каскад).
resolve_order() {
    local root="$1"
    python3 - "$WS" "$root" "$CRATE_PREFIX" <<'PY'
import sys, re, glob, os
ws, root, prefix = sys.argv[1], sys.argv[2], sys.argv[3]
deps = {}
for toml in glob.glob(os.path.join(ws, "crates", "*", "Cargo.toml")):
    text = open(toml).read()
    m = re.search(r'^name *= *"([^"]+)"', text, re.M)
    if not m:
        continue
    name = m.group(1)
    ds = set(re.findall(rf'^({re.escape(prefix)}[a-z0-9_-]+) *=', text, re.M)) - {name}
    deps[name] = ds

if root:
    if root not in deps:
        sys.exit(f"crate {root} not found in workspace")
    # Транзитивные зависящие от root
    dependents = set()
    changed = True
    while changed:
        changed = False
        for c, ds in deps.items():
            if c == root or c in dependents:
                continue
            if root in ds or (ds & dependents):
                dependents.add(c)
                changed = True
    sel = dependents | {root}
else:
    sel = set(deps.keys())

# Топологический порядок внутри sel
order, placed = [], set()
while len(order) < len(sel):
    progressed = False
    for c in sorted(sel - placed):
        if (deps[c] & sel) <= placed:
            order.append(c)
            placed.add(c)
            progressed = True
            break
    if not progressed:  # цикл в графе — публикуем как есть
        order.extend(sorted(sel - placed))
        break
print(" ".join(order))
PY
}

# ── Составляем план ───────────────────────────────────────────────────────────
if $PUBLISH_ALL; then
    PLAN="$(resolve_order "")" || die "не удалось построить план для --publish-all"
elif $CASCADE; then
    PLAN="$(resolve_order "$CRATE")" || die "не удалось построить каскад"
else
    PLAN="$CRATE"
fi

echo ""
if $PUBLISH_ALL; then
    echo -e "${BOLD}Режим:${RESET}    --publish-all (весь воркспейс)"
else
    echo -e "${BOLD}Крейт:${RESET}    $CRATE"
fi
if [[ -z "$BUMP" ]]; then
    echo -e "${BOLD}Бамп:${RESET}     нет (публикуем то, чего ещё нет на crates.io)"
else
    echo -e "${BOLD}Бамп:${RESET}     $BUMP"
fi
if $PUBLISH_ALL || [[ "$PLAN" != "$CRATE" ]]; then
    echo -e "${BOLD}Порядок:${RESET}  $PLAN"
fi
echo ""

run_preflight "$PLAN"
echo ""

# ── Шаг 1: решаем версию/бамп для каждого крейта, в порядке зависимостей ─────
declare -A ORIGINAL_VERSIONS
declare -A NEW_VERSIONS
declare -A SHOULD_PUBLISH
for c in $PLAN; do
    ORIGINAL_VERSIONS[$c]="$(crate_version "$c")"
    if $PUBLISH_ALL || [[ "$c" == "$CRATE" ]]; then
        crate_bump="$BUMP"
    else
        # Зависимые крейты в каскаде: если корень бампается, они бампаются
        # минором (см. заголовок файла); если корень не бампается — тоже нет.
        crate_bump=""
        [[ -n "$BUMP" ]] && crate_bump="--minor"
    fi
    result="$(resolve_crate_action "$c" "$crate_bump")"
    read -r ver action <<< "$result"
    NEW_VERSIONS[$c]="$ver"
    SHOULD_PUBLISH[$c]="$action"
done

# ── Шаг 2: один общий check по воркспейсу ────────────────────────────────────
echo ""
info "cargo check --workspace ..."
if $DRY_RUN; then
    dryrun "cargo check --workspace (пропущено)"
else
    cargo check --offline --workspace --manifest-path "$WS/Cargo.toml" 2>&1 | tail -3
    ok "check пройден"
fi

# ── Шаг 2.5: коммитим бампы версий ────────────────────────────────────────────
# cargo publish отказывается публиковать грязное дерево (без --allow-dirty),
# а бампы версий на шаге 1 правят Cargo.toml на диске, ничего не коммитя —
# отсюда и падение "to proceed despite this... pass --allow-dirty". Коммитим
# сюда, а не --allow-dirty на публикации: --allow-dirty опубликовал бы дерево
# как есть в момент публикации, включая любые посторонние незакоммиченные
# правки, а не только сами бампы.
BUMPED=()
for c in $PLAN; do
    [[ "${ORIGINAL_VERSIONS[$c]}" != "${NEW_VERSIONS[$c]}" ]] && BUMPED+=("$c v${NEW_VERSIONS[$c]}")
done

echo ""
if $DRY_RUN; then
    dryrun "git commit (пропущено)"
elif [[ ${#BUMPED[@]} -eq 0 ]]; then
    info "Версии не менялись — коммитить нечего."
elif git -C "$WS" diff --quiet -- Cargo.toml 'crates/*/Cargo.toml'; then
    warn "Версии изменились в памяти скрипта, но diff по Cargo.toml пуст — пропускаю коммит."
else
    info "Коммичу бамп версий (${#BUMPED[@]})..."
    git -C "$WS" add -- Cargo.toml crates/*/Cargo.toml
    if [[ ${#BUMPED[@]} -eq 1 ]]; then
        git -C "$WS" commit -q -m "chore(release): bump ${BUMPED[0]}"
    else
        {
            echo "chore(release): bump ${#BUMPED[@]} crate versions"
            echo ""
            for b in "${BUMPED[@]}"; do echo "- $b"; done
        } | git -C "$WS" commit -q -F -
    fi
    ok "Закоммичено: $(git -C "$WS" rev-parse --short HEAD)"
fi

# ── Шаг 3: публикация цепочки ─────────────────────────────────────────────────
echo ""
if $NO_PUBLISH; then
    warn "--no-publish: пропускаю cargo publish"
elif $DRY_RUN; then
    for c in $PLAN; do
        if [[ "${SHOULD_PUBLISH[$c]}" == "publish" ]]; then
            dryrun "cargo publish -p $c   # v${NEW_VERSIONS[$c]}"
        else
            dryrun "cargo publish -p $c   # v${NEW_VERSIONS[$c]} — уже опубликован, пропуск"
        fi
    done
else
    for c in $PLAN; do
        if [[ "${SHOULD_PUBLISH[$c]}" == "publish" ]]; then
            publish_with_retry "$c" "${NEW_VERSIONS[$c]}"
        else
            warn "$c v${NEW_VERSIONS[$c]} уже опубликован — пропускаю"
        fi
    done
fi

# ── Шаг 4: git-теги + GitHub-релизы для опубликованного ──────────────────────
# Best-effort и неблокирующий: если этот шаг вообще запускается, значит шаг 3
# (публикация) выше отработал без `die` — это и есть транзакционность, а не
# отдельная проверка. scripts/gh_release.sh сверяется с crates.io напрямую
# (а не с тем, что бампнул именно этот запуск), так что он идемпотентно
# доделывает теги/релизы и для прошлых успешных, но не дотегированных
# публикаций — включая тот случай, когда прошлый запуск упал на публикации
# уже после коммита бампа.
echo ""
if $NO_GH_RELEASE; then
    warn "--no-gh-release: пропускаю git-теги/GitHub-релизы"
elif $NO_PUBLISH || $DRY_RUN; then
    info "Пропускаю git-теги/GitHub-релизы (--no-publish/--dry-run)"
elif ! command -v gh >/dev/null 2>&1; then
    warn "gh (GitHub CLI) не найден — пропускаю. Запусти ./scripts/gh_release.sh отдельно, когда он появится."
else
    info "Создаю git-теги и GitHub-релизы для опубликованного (scripts/gh_release.sh) ..."
    if "$SCRIPT_DIR/scripts/gh_release.sh"; then
        ok "Теги/релизы готовы."
    else
        warn "scripts/gh_release.sh завершился с ошибкой — публикация на crates.io уже прошла, но теги/релизы могли остаться неполными. Перезапусти ./scripts/gh_release.sh отдельно."
    fi
fi

# ── Итог ──────────────────────────────────────────────────────────────────────
echo ""
if $DRY_RUN; then
    echo -e "${YELLOW}[dry-run]${RESET} Ничего не изменено. Убери --dry-run для реального запуска."
else
    echo -e "${GREEN}${BOLD}Готово!${RESET}"
    for c in $PLAN; do
        echo -e "  $c → ${NEW_VERSIONS[$c]}  https://crates.io/crates/$c/${NEW_VERSIONS[$c]}"
    done
fi
echo ""
