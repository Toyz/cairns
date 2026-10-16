//! The entry list, the site's front page.

use super::*;

/// The index: every entry, with the chips and the search box over it.
pub fn home(log: &Log) -> String {
    let base = log.project.base_url.trim_end_matches('/');

    // The filters live in the rail rather than above the list: they are
    // navigation, they are the same on every visit, and stacking them over the
    // entries pushed the first one off the fold.
    let mut chips = String::from("<div class=\"filters\">\n<p class=\"rail-label\">Areas</p>\n");
    // Where the time goes, beside how many entries each area has - once any
    // entry records how long it took. An entry in two areas counts in both:
    // the work was in both.
    for area in log.areas.iter().filter(|area| area.count > 0) {
        let time = total_took(
            log.entries
                .iter()
                .filter(|entry| entry.areas.contains(&area.name)),
        );
        let _ = writeln!(
            chips,
            "<button class=\"chip\" data-area=\"{name}\" aria-pressed=\"false\" \
             title=\"{about}{hover}\"><span>{name}</span><span class=\"count\">{shown}{count}</span></button>",
            name = escape(&area.name),
            about = escape(&area.about),
            hover = time
                .as_deref()
                .map(|t| format!(" - {t} recorded"))
                .unwrap_or_default(),
            shown = time
                .as_deref()
                .map(|t| format!("<span class=\"area-time\">{t}</span>"))
                .unwrap_or_default(),
            count = area.count
        );
    }
    chips.push_str("</div>\n");

    let mut body = String::from("<div class=\"toolbar\">\n");
    body.push_str(&search_field(&format!(
        "Search {} entries",
        log.entries.len()
    )));
    // Newest first is what someone checking back wants, and it is the order the
    // page ships in so it holds with scripting off.
    //
    // Icons rather than words: four words beside the search field read as a
    // second toolbar. Each keeps its name for a screen reader and on hover.
    let icon = |paths: &str| {
        format!(
            "<svg viewBox=\"0 0 16 16\" aria-hidden=\"true\" focusable=\"false\" fill=\"none\" \
             stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\" \
             stroke-linejoin=\"round\">{paths}</svg>"
        )
    };
    let _ = writeln!(
        body,
        "<div class=\"switch\" role=\"group\" aria-label=\"Order\">\n\
         <button data-sort=\"new\" aria-pressed=\"true\" aria-label=\"Newest first\" \
         title=\"Newest first\">{newest}</button>\n\
         <button data-sort=\"old\" aria-pressed=\"false\" aria-label=\"Oldest first\" \
         title=\"Oldest first\">{oldest}</button>\n</div>\n\
         <div class=\"switch\" role=\"group\" aria-label=\"Density\">\n\
         <button data-density=\"detailed\" aria-pressed=\"true\" aria-label=\"Detailed\" \
         title=\"Detailed\">{detailed}</button>\n\
         <button data-density=\"compact\" aria-pressed=\"false\" aria-label=\"Compact\" \
         title=\"Compact\">{compact}</button>\n</div>\n</div>\n\
         <p id=\"status\"></p>\n<div class=\"days\" id=\"entries\">",
        // An arrow beside lines that shrink the way it points.
        newest =
            icon("<path d=\"M4 2.5v11M1.75 11.25 4 13.5l2.25-2.25M9 3.5h5.5M9 8h4M9 12.5h2.5\"/>"),
        oldest =
            icon("<path d=\"M4 13.5v-11M1.75 4.75 4 2.5l2.25 2.25M9 3.5h2.5M9 8h4M9 12.5h5.5\"/>"),
        // A title over a summary, twice; and four plain lines.
        detailed = icon("<path d=\"M2 3h12M2 6h8\"/><path d=\"M2 10.5h12M2 13.5h8\" />"),
        compact = icon("<path d=\"M2 3h12M2 6.33h12M2 9.67h12M2 13h12\"/>"),
    );

    // Under a heading per day rather than a date on every row. A log that runs
    // to hundreds of entries is written a dozen a day, and the same date
    // printed twelve times over is noise between titles. A day is a run of
    // entries in number order, so a date out of sequence shows up as its own
    // short group instead of being silently folded into another.
    let newest_first: Vec<&LogEntry> = log.entries.iter().rev().collect();
    for day in newest_first.chunk_by(|a, b| a.date == b.date) {
        let date = day[0].date;
        let _ = writeln!(
            body,
            "<section class=\"day\" id=\"d{date}\">\n<h2 class=\"day-label\"><time datetime=\"{date}\">{long}</time>\
             <span class=\"day-count\">{count}</span>{took}</h2>\n<ul class=\"entries\">",
            long = long_date(date),
            count = day.len(),
            took = total_took(day.iter().copied())
                .map(|t| format!("<span class=\"day-took\">{t}</span>"))
                .unwrap_or_default()
        );
        for entry in day {
            let _ = writeln!(
                body,
                "<li data-n=\"{n}\" data-areas=\"{slugs}\">\n\
                 <a class=\"row\" href=\"{path}/\">\n\
                 <span class=\"no\">{n}</span>\n<span class=\"row-main\">\n\
                 <span class=\"row-title\" title=\"{title}\">{title}</span>",
                n = entry.number,
                slugs = escape(&entry.areas.join(" ")),
                path = escape(&path_of(entry)),
                title = escape(&entry.title)
            );
            if let Some(summary) = &entry.summary {
                let _ = writeln!(
                    body,
                    "<span class=\"summary\">{}</span>",
                    escape(&plain_md(summary))
                );
            }
            let _ = writeln!(
                body,
                "<span class=\"meta\"><span class=\"areas\">{areas}</span>{took}</span>\n\
                 </span>\n</a>\n</li>",
                areas = escape(&entry.areas.join(" \u{00b7} ")),
                took = entry
                    .took_minutes
                    .map(|m| format!(
                        "<span class=\"took\">{}</span>",
                        cairns_core::entry::format_duration(m)
                    ))
                    .unwrap_or_default()
            );
        }
        body.push_str("</ul>\n</section>\n");
    }

    body.push_str("</div>\n<script src=\"search.js\" defer></script>\n");

    let description = if log.project.description.is_empty() {
        format!("{} entries.", log.entries.len())
    } else {
        plain_md(&log.project.description)
    };
    shell(
        log,
        Page {
            title: &log.project.name,
            description: &description,
            url: base,
            rel: "./",
            nav: Nav::Entries,
            aside: &chips,
            contents: &glance(log),
            body: &body,
        },
    )
}

/// The log at a glance, beside the list: how much there is, how much is still
/// open, and when it was written.
///
/// A list of three hundred entries says nothing about its own shape. Whether a
/// log is alive, whether it came in one burst or a steady drip, and how much of
/// it is unfinished are the first things a visitor wants to know, and none of
/// them could be read off the page.
pub(crate) fn glance(log: &Log) -> String {
    let (Some(first), Some(last)) = (log.entries.first(), log.entries.last()) else {
        return String::new();
    };
    let open = log
        .entries
        .iter()
        .filter(|entry| entry.still_unknown.is_some() && entry.resolved_by.is_empty())
        .count();

    let mut out = String::from(
        "<div class=\"glance\">\n<p class=\"rail-label\">This log</p>\n<dl class=\"glance-stats\">\n",
    );
    let _ = writeln!(
        out,
        "<div><dt>Entries</dt><dd>{}</dd></div>\n\
         <div><dt>Open</dt><dd><a href=\"open/\">{open}</a></dd></div>",
        log.entries.len()
    );
    if let Some(total) = total_took(log.entries.iter()) {
        let clocked = log
            .entries
            .iter()
            .filter(|e| e.took_minutes.is_some())
            .count();
        let _ = writeln!(
            out,
            "<div class=\"wide\" title=\"{clocked} of {} entries record how long they took\">\
             <dt>Time</dt><dd>{total}</dd></div>",
            log.entries.len()
        );
    }
    out.push_str("</dl>\n");

    // One bar per day across the whole span, empty days included - a gap is
    // part of the shape. A span too long for a bar a day is drawn by week, or
    // by month, so the strip stays readable rather than becoming a smear.
    let start = day_number(first.date.min(last.date));
    let end = day_number(first.date.max(last.date));
    let span = end - start + 1;
    let width = match span {
        ..=120 => 1,
        121..=730 => 7,
        _ => 30,
    };
    // `span` is at least one day: the last entry is not before the first.
    let buckets = usize::try_from((span + width - 1) / width).unwrap_or(1);
    let mut counts = vec![0usize; buckets];
    let mut firsts: Vec<Option<cairns_core::Date>> = vec![None; buckets];
    for entry in &log.entries {
        let at = usize::try_from((day_number(entry.date) - start) / width).unwrap_or(usize::MAX);
        if let Some(count) = counts.get_mut(at) {
            *count += 1;
            let first_day = &mut firsts[at];
            if first_day.is_none_or(|day| entry.date < day) {
                *first_day = Some(entry.date);
            }
        }
    }
    let busiest = counts.iter().copied().max().unwrap_or(1).max(1);
    let unit = match width {
        1 => "day",
        7 => "week",
        _ => "month",
    };
    let _ = writeln!(
        out,
        "<div class=\"spark\" role=\"img\" aria-label=\"Entries per {unit}, busiest {busiest}\">"
    );
    for (count, day) in counts.iter().zip(&firsts) {
        match day {
            Some(day) => {
                let label = if width == 1 {
                    format!("{}: {count}", long_date(*day))
                } else {
                    format!("{unit} of {}: {count}", long_date(*day))
                };
                let _ = writeln!(
                    out,
                    "<a href=\"#d{day}\" style=\"--h:{pct}%\" title=\"{label}\"></a>",
                    pct = (count * 100).div_ceil(busiest).max(4),
                    label = escape(&label)
                );
            }
            None => out.push_str("<span></span>\n"),
        }
    }
    let _ = writeln!(
        out,
        "</div>\n<p class=\"spark-axis\"><span>{}</span><span>{}</span></p>",
        short_date(first.date.min(last.date)),
        short_date(first.date.max(last.date))
    );
    out.push_str(
        "<p class=\"keys\"><kbd>/</kbd> search <kbd>j</kbd><kbd>k</kbd> move \
         <kbd>\u{21b5}</kbd> open</p>\n</div>\n",
    );
    out
}
