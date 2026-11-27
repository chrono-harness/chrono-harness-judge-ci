//! JSON interface used by native CI providers in the checked host.
#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use crate::{
    originals::{
        ArtifactTransport, FullManifest, FullReportInput, Original, original_path, read_bounded,
    },
    wire::{Scope, Unit},
};

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
enum Query {
    Schedule {
        profile: String,
        base: Option<String>,
        candidate: String,
        initial: bool,
    },
    FullSchedule {
        profile: String,
        base: String,
        candidate: String,
    },
    Collect {
        profile: String,
        manifest: String,
        base: Option<String>,
        candidate: String,
        initial: bool,
    },
    FullCollect {
        profile: String,
        context: String,
        scope: Option<Scope>,
        base: String,
        candidate: String,
    },
    UnitOriginals {
        profile: String,
        profile_sha256: String,
        unit: String,
        report: Original,
        transport: ArtifactTransport,
    },
}

#[cfg(not(verus_keep_ghost))]
pub fn query(root: &Path, input: &[u8]) -> Result<Value, String> {
    match decode::<Query>(input)? {
        Query::Schedule {
            profile,
            base,
            candidate,
            initial,
        } => {
            crate::files::no_symlink_parents(root, &profile)?;
            scheduling_requirements(root, &profile, base, candidate, initial)
        }
        Query::FullSchedule {
            profile,
            base,
            candidate,
        } => {
            crate::files::no_symlink_parents(root, &profile)?;
            full_scheduling_requirements(root, &profile, &base, &candidate)
        }
        Query::Collect {
            profile,
            manifest,
            base,
            candidate,
            initial,
        } => {
            crate::files::no_symlink_parents(root, &profile)?;
            units::artifact_path(&manifest)?;
            collection_requirements(root, &profile, &manifest, base, candidate, initial)
        }
        Query::FullCollect {
            profile,
            context,
            scope,
            base,
            candidate,
        } => {
            crate::files::no_symlink_parents(root, &profile)?;
            crate::files::no_symlink_parents(root, &context)?;
            let config = crate::runtime_session::Session::selected_config(root, &profile)?;
            let block: Option<Value> = crate::runtime_session::Session::query(
                root,
                "judge-evidence-v1",
                object!({"operation":"full-execution-units","profile":config}),
            )?;
            let block = block.ok_or("full provider requires registered full units")?;
            let registered: BTreeMap<String, Unit> =
                serde_json::from_value(block["units"].clone()).map_err(|e| e.to_string())?;
            let limit = block["collection_limits"]["report_bytes"]
                .as_u64()
                .ok_or("full report bound")?;
            full_collection_at(
                root,
                &profile,
                &base,
                &candidate,
                Path::new(&context),
                scope,
                &|units| {
                    let mut reports = Vec::new();
                    for unit in units {
                        let input = registered
                            .get(unit)
                            .ok_or("migration unit missing provider registration")?;
                        let raw = read_bounded(root, &input.report_path, limit)?;
                        reports.push(FullReportInput {
                            unit: unit.clone(),
                            path: input.report_path.clone(),
                            sha256: sha256(&raw),
                            runner_sha256: None,
                            judge_sha256: None,
                            artifacts: None,
                        });
                    }
                    Ok(FullManifest {
                        schema: "chrono-full-collection/v1".into(),
                        reports,
                    })
                },
            )
        }
        Query::UnitOriginals {
            profile,
            profile_sha256,
            unit,
            report,
            transport,
        } => {
            let raw = fs::read(crate::files::no_symlink_parents(root, &profile)?)
                .map_err(|e| e.to_string())?;
            if !profile_matches(&sha256(&raw), &profile_sha256) {
                return Err("collection check profile changed".into());
            }
            let config: CheckConfig = decode(&raw)?;
            let registered: BTreeMap<String, Unit> =
                serde_json::from_value(config.policy["units"].clone())
                    .map_err(|e| e.to_string())?;
            if !report_path_matches(
                registered.get(&unit).map(|u| u.report_path.as_str()),
                &report.path,
            ) {
                return Err("unit report differs from profile registration".into());
            }
            let limit: CollectionLimits = config
                .policy
                .get("collection_limits")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| e.to_string())?
                .unwrap_or_default();
            let mapped = |original: &Original, limit| {
                read_bounded(
                    root,
                    &original_path(root, original, Some(&transport))?,
                    limit,
                )
            };
            let bytes = mapped(&report, limit.report_bytes)?;
            let accepted = admitted_report(&bytes, &sha256(&bytes), &report.sha256)
                .ok_or_else(|| format!("unit {unit} report digest mismatch"))?;
            unit_originals(root, &config, &unit, accepted, Some(&transport), mapped)?;
            Ok(object!({"unit":unit,"report_sha256":report.sha256}))
        }
    }
}

#[cfg(verus_keep_ghost)]
use super::verified_text::equal_text;
#[cfg(not(verus_keep_ghost))]
use crate::verified_collection::verified_text::equal_text;
use vstd::prelude::*;

verus! {
/// Identifies the exact profile before its bytes can be decoded.
pub fn profile_matches(observed: &str, declared: &str) -> (ok: bool)
    ensures ok == (observed@ == declared@),
{
    equal_text(observed, declared)
}

/// The report must name the path registered for this exact unit.
pub fn report_path_matches(registered: Option<&str>, declared: &str) -> (ok: bool)
    ensures ok == match registered { Some(path) => path@ == declared@, None => false },
{
    match registered {
        Some(path) => equal_text(path, declared),
        None => false,
    }
}

/// Only deliver bytes to the collector after their observed identity matches.
pub fn admitted_report<'a>(bytes: &'a [u8], observed: &str, declared: &str)
    -> (out: Option<&'a [u8]>)
    ensures match out {
        Some(data) => observed@ == declared@ && data == bytes,
        None => observed@ != declared@,
    },
{
    if equal_text(observed, declared) { Some(bytes) } else { None }
}

}
