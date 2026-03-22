import init, { parse_to_html } from '../../src-web/pkg/src_web.js';

async function main() {
    await init();
    
    const editor = document.getElementById('editor') as HTMLTextAreaElement;
    const preview = document.getElementById('preview') as HTMLDivElement;

    const render = () => {
        const markdown = editor.value;
        const html = parse_to_html(markdown);
        preview.innerHTML = html;
    };

    editor.addEventListener('input', render);
    
    // Initial render
    editor.value = "# Hello Gloss!\n\nThis is a test of `[漢字/かんじ]` and `{Gloss/Test}`.";
    render();
}

main();
