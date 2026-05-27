pub const SYSTEM_PROMPT: &str = "You are Neura, an intelligent AI assistant built into the Neura Search browser. You help users understand web content, answer questions, summarize pages, and assist with research. Be concise, accurate, and helpful. When given page context, focus your answers on that content unless the user asks otherwise.";

pub fn summarize_prompt(style: &str, page_title: &str, page_url: &str, page_text: &str) -> String {
    let instruction = match style {
        "short" => "Provide a brief 2-3 sentence summary.",
        "detailed" => "Provide a detailed summary covering all key points.",
        "bullet_points" => "Provide a summary as a concise bullet-point list.",
        "study_notes" => "Provide structured study notes with headers and key concepts.",
        _ => "Provide a clear, concise summary.",
    };

    format!(
        "Page: {}\nURL: {}\n\n{}\n\nPage content:\n{}\n\n---\n{}",
        page_title,
        page_url,
        instruction,
        &page_text[..page_text.len().min(8000)],
        instruction
    )
}

pub fn explain_prompt(page_title: &str, page_text: &str) -> String {
    format!(
        "Explain the following page content in simple, clear terms. Page: {}\n\nContent:\n{}",
        page_title,
        &page_text[..page_text.len().min(6000)]
    )
}

pub fn key_points_prompt(page_title: &str, page_text: &str) -> String {
    format!(
        "Extract the 5-7 most important key points from this page. Be concise.\nPage: {}\n\nContent:\n{}",
        page_title, &page_text[..page_text.len().min(6000)]
    )
}

pub fn action_items_prompt(page_title: &str, page_text: &str) -> String {
    format!(
        "Extract any action items, tasks, or next steps mentioned in this content.\nPage: {}\n\nContent:\n{}",
        page_title, &page_text[..page_text.len().min(6000)]
    )
}

pub fn page_context_prefix(page_title: &str, page_url: &str, page_text: &str) -> String {
    format!(
        "Current page context:\nTitle: {}\nURL: {}\nContent (truncated):\n{}\n\n---\nUser question: ",
        page_title, page_url, &page_text[..page_text.len().min(4000)]
    )
}
