use codex_skilltree::catalog::Catalog;
use serde_json::json;
use std::path::PathBuf;

fn run() -> Result<(), String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let root = std::env::var_os("CODEX_SKILLTREE_ROOT")
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir().map_err(|e| e.to_string())?);
    let mut budget = None;
    if let Some(index) = args.iter().position(|a| a == "--budget") {
        budget = Some(
            args.get(index + 1)
                .ok_or("--budget needs a number")?
                .parse::<usize>()
                .map_err(|_| "Budget must be a positive number")?,
        );
        args.drain(index..=index + 1);
    }
    if args.is_empty() || args[0] == "--help" {
        println!(
            "skilltree classes | tree [class-id] | search <words> | route <task> | load <skill-id>... [--budget N]\nUse /skilltree inside ./codex-skilltree for the native editor."
        );
        return Ok(());
    }
    let catalog = Catalog::open(&root)?;
    let budget = budget.unwrap_or(catalog.config.default_budget).min(10_000);
    if budget == 0 {
        return Err("Budget must be positive".into());
    }
    let card = |skill: &codex_skilltree::catalog::Skill| {
        json!({
            "id": skill.id, "name": skill.name, "description": skill.description,
            "class": skill.class, "tree": skill.tree, "tags": skill.tags,
            "requires": skill.requires, "estimated_tokens": skill.bytes.div_ceil(4)
        })
    };
    let value = match args[0].as_str() {
        "classes" => json!(catalog.config.classes),
        "tree" => json!(
            catalog
                .skills
                .iter()
                .filter(|s| args.get(1).is_none_or(|id| id == &s.class))
                .map(card)
                .collect::<Vec<_>>()
        ),
        "search" => {
            let query = args[1..].join(" ").to_lowercase();
            json!(
                catalog
                    .skills
                    .iter()
                    .filter(|s| {
                        let haystack =
                            format!("{} {} {} {}", s.id, s.name, s.description, s.tags.join(" "))
                                .to_lowercase();
                        query.split_whitespace().all(|word| haystack.contains(word))
                    })
                    .map(card)
                    .collect::<Vec<_>>()
            )
        }
        "route" => {
            let task = args[1..].join(" ");
            let ids = catalog.suggest(&task, budget)?;
            json!({"selected":ids.iter().map(|id| catalog.resolve(id).map(card)).collect::<Result<Vec<_>, _>>()?,
                "load_order":catalog.expand(&ids)?.iter().map(|s| &s.id).collect::<Vec<_>>(), "budget":budget,
                "estimated_tokens":if ids.is_empty() { 0 } else { catalog.estimate(&ids)? }})
        }
        "load" => {
            println!(
                "{}",
                catalog.load(&args[1..], "Use these skills for the current task.", budget)?
            );
            return Ok(());
        }
        command => return Err(format!("Unknown command: {command}")),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
