-- Pandoc Lua filter: convert svgdx-tagged fenced code blocks into inline SVG
-- or image links backed by explicit output files.

-- Name of the environment variable that can override the svgdx binary.
-- If not set, the filter assumes 'svgdx' is on the PATH.
local SVGDX_ENV = "SVGDX_BIN"
local svgdx_cmd = os.getenv(SVGDX_ENV) or "svgdx"

local function exec_svgdx(args, stdin)
  return pcall(pandoc.pipe, svgdx_cmd, args, stdin)
end

-- svgdx availability check (runs at filter load time)
local svgdx_ok, svgdx_err = exec_svgdx({ "--version" }, "")
if not svgdx_ok then
  error("could not run svgdx: " .. svgdx_err)
end

local function html_escape(s)
  return (s:gsub("&", "&amp;"):gsub("<", "&lt;"):gsub(">", "&gt;"))
end

-- Format an error message as a red-bordered HTML <div>.
local function error_format(msg)
  -- escape msg first, then replace newlines with <br/> for HTML
  return string.format(
    '<div style="color: red; border: 5px double red; padding: 1em;">%s</div>',
    html_escape(msg):gsub("\n", "<br/>")
  )
end

-- Error helpers
local function error_block(msg)
  warn("svgdx processing error: " .. msg)
  return pandoc.RawBlock("html", error_format(msg))
end

-- Write bytes to a file path, raising a Lua error on failure.
local function write_file(path, content)
  local tmp = path .. ".tmp"
  local f, err = io.open(tmp, "wb")
  if not f then
    error("Failed to open file '" .. tmp .. "': " .. err)
  end

  local fh, write_err = f:write(content)
  if not fh then
    f:close()
    error("Failed to write file '" .. tmp .. "': " .. write_err)
  end

  local close_ok, close_err = f:close()
  if not close_ok then
    error("Failed to close file '" .. tmp .. "': " .. close_err)
  end

  local rename_ok, rename_err = os.rename(tmp, path)
  if not rename_ok then
    os.remove(tmp)
    error("Failed to rename temp file to '" .. path .. "': " .. rename_err)
  end
end

-- Formats where the output document links to the image file rather than
-- embedding it. The user's relative path is preserved as the image link so
-- it remains readable and editable in the output source.
local linked_formats = {
  html = true, html4 = true, html5 = true, chunkedhtml = true,
  markdown = true, markdown_strict = true, markdown_phpextra = true,
  markdown_github = true, markdown_mmd = true,
  gfm = true, commonmark = true, commonmark_x = true,
  rst = true, mediawiki = true, textile = true,
  dokuwiki = true, xwiki = true, twiki = true, jira = true,
  org = true, muse = true, asciidoc = true, asciidoctor = true,
}

-- Given the user-supplied output= path, return (write_path, link_path):
--   write_path: absolute path where the SVG will be written
--   link_path:  path to embed as the image src in the output document
--
-- For relative paths, the SVG is always written to that path resolved against
-- the output document's directory (so the physical file location is the same
-- regardless of where pandoc is run from or which format is being generated).
--
-- For linked formats (HTML, Markdown, etc) the link keeps the user's original
-- relative path, which the reader resolves from the document's own location.
-- For embedded formats (PDF, DOCX, etc) the link is the absolute write path,
-- so pandoc's pipeline can find the file regardless of its working directory.
-- When pandoc writes to stdout, there is no document location to resolve a
-- relative link against, so the absolute write path is used in all formats.
-- Absolute output= paths are always used verbatim as both write and link.
local function resolve_output_paths(output_path)
  local cwd = pandoc.system.get_working_directory()
  local function make_abs(p)
    if pandoc.path.is_absolute(p) then
      return pandoc.path.normalize(p)
    end
    return pandoc.path.normalize(pandoc.path.join({ cwd, p }))
  end

  if pandoc.path.is_absolute(output_path) then
    local abs = pandoc.path.normalize(output_path)
    return abs, abs
  end

  local out_file = PANDOC_STATE.output_file
  local has_output_file = out_file and out_file ~= "" and out_file ~= "-"
  local base_dir = has_output_file
    and pandoc.path.directory(make_abs(out_file))
    or cwd
  local write_path = pandoc.path.normalize(pandoc.path.join({ base_dir, output_path }))
  local link_path = (has_output_file and linked_formats[FORMAT]) and output_path or write_path
  return write_path, link_path
end


-- Blank lines within inline SVG can confuse Markdown processors into treating
-- subsequent content as a new block; see https://spec.commonmark.org/0.31.2/#html-blocks
local function blank_line_remover(s)
  local lines = {}
  for line in (s .. "\n"):gmatch("([^\n]*)\n") do
    if not line:match("^%s*$") then
      table.insert(lines, line)
    end
  end
  return table.concat(lines, "\n")
end

-- CodeBlock filter - called by Pandoc on fenced code blocks
local function CodeBlock(block)
  -- Only handle blocks tagged with the svgdx class: '```svgdx' and
  -- '```{.svgdx <attributes>}'
  if not block.classes:includes("svgdx") then
    return nil
  end

  -- Pipe the block content through svgdx executable.
  local ok, svg = exec_svgdx({ "--auto-style-mode", "inline" }, block.text)
  if not ok then
    return error_block(svg)
  end

  -- If the block has an 'output' attribute, write the SVG to that file and
  -- return an image link.
  local output_path = block.attributes.output
  if output_path and output_path ~= "" then
    local write_path, link_path = resolve_output_paths(output_path)
    local write_ok, write_err = pcall(write_file, write_path, svg)
    if not write_ok then
      return error_block(write_err)
    end
    return pandoc.Para({ pandoc.Image({}, link_path, "") })
  end

  -- Strip blank lines in inline SVG for valid Markdown
  return pandoc.RawBlock("html", blank_line_remover(svg))
end

-- Return the filter table to Pandoc
return {
  { CodeBlock = CodeBlock },
}
