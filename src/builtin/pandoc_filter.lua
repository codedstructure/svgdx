local path = pandoc.path
local system = pandoc.system

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
  local svg = run_pipe(svgdx_bin(), { "-i", "-" }, content)
  return pandoc.RawBlock("html", svg)
end

local function render_svg_file(content, relative_name)
  local absolute_name = resolve_target(relative_name)
  ensure_parent_directory(absolute_name)
  run_pipe(svgdx_bin(), { "-i", "-", "-o", absolute_name }, content)
  return pandoc.Para({ pandoc.Image({}, relative_name) })
end

local function render_png_file(content, relative_name)
  local absolute_name = resolve_target(relative_name)
  ensure_parent_directory(absolute_name)

  return system.with_temporary_directory("svgdx-", function(temp_dir)
    local temp_svg = path.join({ temp_dir, "image.svg" })
    run_pipe(svgdx_bin(), { "-i", "-", "-o", temp_svg }, content)
    run_pipe("rsvg-convert", { "-o", absolute_name, temp_svg }, "")
    return pandoc.Para({ pandoc.Image({}, relative_name) })
  end)
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
