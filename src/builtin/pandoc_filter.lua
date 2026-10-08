-- Pandoc Lua filter for processing fenced code blocks with the `svgdx` class,
-- rendering them as inline SVG, SVG files, or PNG files.
--
-- Markdown input syntax for `svgdx` code blocks:
-- ```svgdx
-- <!-- svgdx content here will be rendeded as inline SVG in the output -->
-- ```
--
-- ```svgdx { filename = "output.svg" }
-- <!-- svgdx content here will be rendeded to 'output.svg'.
--      Filenames are relative to the output directory. -->
-- ```
--
-- By providing a filename with a .png extension, the content is passed through
-- the `rsvg-convert` tool to generate a PNG file.

local path = pandoc.path
local system = pandoc.system

local function svgdx_args(...)
  local args = {
    "--auto-style-mode", "css-html",
    "-i", "-",
  }

  for i = 1, select("#", ...) do
    args[#args + 1] = select(i, ...)
  end

  return args
end

local function svgdx_bin()
  local bin = os.getenv("SVGDX_BIN")
  if bin ~= nil and bin ~= "" then
    return bin
  end
  return "svgdx"
end

local function has_class(classes, name)
  for _, class_name in ipairs(classes) do
    if class_name == name then
      return true
    end
  end
  return false
end

local function output_directory()
  local output_name = PANDOC_STATE.output_file
  if output_name == nil or output_name == "" or output_name == "-" then
    return "."
  end

  local dir = path.directory(output_name)
  if dir == "" then
    return "."
  end
  return dir
end

local function resolve_target(filename)
  return path.normalize(path.join({ output_directory(), filename }))
end

local function ensure_parent_directory(filename)
  local dir = path.directory(filename)
  if dir ~= nil and dir ~= "" and dir ~= "." then
    system.make_directory(dir, true)
  end
end

local function run_pipe(program, args, input)
  local ok, result = pcall(pandoc.pipe, program, args, input or "")
  if ok then
    return result
  end
  error(result)
end

local function render_inline_svg(content)
  local svg = run_pipe(svgdx_bin(), svgdx_args(), content)
  return pandoc.RawBlock("html", svg)
end

local function render_svg_file(content, relative_name)
  local absolute_name = resolve_target(relative_name)
  ensure_parent_directory(absolute_name)
  run_pipe(svgdx_bin(), svgdx_args("-o", absolute_name), content)
  return pandoc.Para({ pandoc.Image({}, relative_name) })
end

local function render_png_file(content, relative_name)
  local absolute_name = resolve_target(relative_name)
  ensure_parent_directory(absolute_name)
  local temp_svg = run_pipe(svgdx_bin(), svgdx_args(), content)
  run_pipe("rsvg-convert", { "--dpi-x", "300", "--dpi-y", "300", "-o", absolute_name }, temp_svg)
  local w = pandoc.image.size(temp_svg).width
  return pandoc.Para({ pandoc.Image({}, relative_name, "", {width = w}) })
end

function CodeBlock(block)
  if not has_class(block.classes, "svgdx") then
    return nil
  end

  local filename = block.attributes.filename
  if filename == nil or filename == "" then
    return render_inline_svg(block.text)
  end

  local _, extension = path.split_extension(filename)
  extension = string.lower(extension)

  if extension == ".svg" then
    return render_svg_file(block.text, filename)
  end

  if extension == ".png" then
    return render_png_file(block.text, filename)
  end

  error(string.format(
    "svgdx pandoc filter only supports filename values ending in .svg or .png: %s",
    filename
  ))
end
