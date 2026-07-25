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
  local ok, svg = exec_svgdx({}, block.text)
  if not ok then
    return error_block(svg)
  end

  -- If the block has an 'output' attribute, write the SVG to that file and
  -- return an image link.
  local output_path = block.attributes.output
  if output_path and output_path ~= "" then
    local write_ok, write_err = pcall(write_file, output_path, svg)
    if not write_ok then
      return error_block(write_err)
    end
    return pandoc.Para({ pandoc.Image({}, output_path, "") })
  end

  -- Strip blank lines in inline SVG for valid Markdown
  return pandoc.RawBlock("html", blank_line_remover(svg))
end

-- Return the filter table to Pandoc
return {
  { CodeBlock = CodeBlock },
}
