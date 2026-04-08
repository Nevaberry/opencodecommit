#!/usr/bin/env bash

occ_worktree_repo_root() {
  local script_dir
  script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
  cd -- "${script_dir}/../.." && pwd -P
}

occ_git_common_dir() {
  local repo_path="$1"
  local common_dir

  common_dir=$(git -C "$repo_path" rev-parse --git-common-dir)
  if [[ "$common_dir" == /* ]]; then
    (cd -- "$common_dir" && pwd -P)
  else
    (cd -- "$repo_path" && cd -- "$common_dir" && pwd -P)
  fi
}

occ_list_worktrees() {
  local repo_root="${1:-$(occ_worktree_repo_root)}"
  local path=""
  local branch=""
  local line=""

  while IFS= read -r line || [[ -n "$line" ]]; do
    case "$line" in
      "worktree "*) path=${line#worktree } ;;
      "branch refs/heads/"*) branch=${line#branch refs/heads/} ;;
      "branch "*) branch=${line#branch } ;;
      "")
        if [[ -n "$path" ]]; then
          printf '%s\t%s\n' "$path" "$branch"
        fi
        path=""
        branch=""
        ;;
    esac
  done < <(git -C "$repo_root" worktree list --porcelain && printf '\n')
}

occ_list_worktrees_pretty() {
  local repo_root="${1:-$(occ_worktree_repo_root)}"
  local path=""
  local branch=""
  local label=""

  while IFS=$'\t' read -r path branch; do
    label=${branch:-$(basename "$path")}
    printf '%-24s %s\n' "$label" "$path"
  done < <(occ_list_worktrees "$repo_root")
}

occ_current_repo_worktree() {
  local repo_root="$1"
  local current_top=""
  local repo_common=""
  local current_common=""

  repo_common=$(occ_git_common_dir "$repo_root")

  if ! current_top=$(git -C "$PWD" rev-parse --show-toplevel 2>/dev/null); then
    printf '%s\n' "$repo_root"
    return 0
  fi

  if ! current_common=$(occ_git_common_dir "$current_top" 2>/dev/null); then
    printf '%s\n' "$repo_root"
    return 0
  fi

  if [[ "$current_common" == "$repo_common" ]]; then
    printf '%s\n' "$current_top"
  else
    printf '%s\n' "$repo_root"
  fi
}

occ_resolve_worktree() {
  local selector="${1:-}"
  local repo_root="${2:-$(occ_worktree_repo_root)}"
  local repo_common=""
  local path=""
  local branch=""
  local candidate=""
  local -a matches=()

  repo_common=$(occ_git_common_dir "$repo_root")

  if [[ -z "$selector" ]]; then
    occ_current_repo_worktree "$repo_root"
    return 0
  fi

  if [[ -e "$selector" ]]; then
    if candidate=$(git -C "$selector" rev-parse --show-toplevel 2>/dev/null); then
      if [[ "$(occ_git_common_dir "$candidate")" == "$repo_common" ]]; then
        printf '%s\n' "$candidate"
        return 0
      fi
    fi
  fi

  candidate="${repo_root}/.worktrees/${selector}"
  if [[ -d "$candidate" ]]; then
    (cd -- "$candidate" && pwd -P)
    return 0
  fi

  while IFS=$'\t' read -r path branch; do
    if [[ "$path" == "$selector" || "$(basename "$path")" == "$selector" || "$branch" == "$selector" ]]; then
      matches+=("$path")
    fi
  done < <(occ_list_worktrees "$repo_root")

  if [[ ${#matches[@]} -eq 1 ]]; then
    printf '%s\n' "${matches[0]}"
    return 0
  fi

  if [[ ${#matches[@]} -gt 1 ]]; then
    printf 'ambiguous worktree selector: %s\n' "$selector" >&2
    return 1
  fi

  printf 'unknown worktree: %s\n' "$selector" >&2
  return 1
}

occ_worktree_label() {
  local worktree_path="$1"
  local repo_root="${2:-$(occ_worktree_repo_root)}"
  local path=""
  local branch=""

  while IFS=$'\t' read -r path branch; do
    if [[ "$path" == "$worktree_path" ]]; then
      printf '%s\n' "${branch:-$(basename "$path")}"
      return 0
    fi
  done < <(occ_list_worktrees "$repo_root")

  printf '%s\n' "$(basename "$worktree_path")"
}

occ_slugify() {
  printf '%s' "$1" | sed -E 's#[^A-Za-z0-9._-]+#-#g; s#-+#-#g; s#(^-+|-+$)##g'
}

occ_dev_state_root() {
  local worktree_path="$1"
  local namespace="$2"
  local repo_root="${3:-$(occ_worktree_repo_root)}"
  local common_dir=""
  local label=""

  common_dir=$(occ_git_common_dir "$worktree_path")
  label=$(occ_worktree_label "$worktree_path" "$repo_root")

  printf '%s/%s/%s\n' "$common_dir/dev" "$namespace" "$(occ_slugify "$label")"
}
