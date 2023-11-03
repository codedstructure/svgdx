pub mod utils;
use utils::compare;

#[test]
fn test_transform_full_svg() {
    let input = r##"<svg>
  <style>
    <![CDATA[
    * { stroke-width: 0.2; fill: none; stroke: black; }
    ]]>
  </style>
  <rect xy="0" wh="5" id="a"/>
  <rect xy="@bl 0 2" wh="5" repeat="3" />
  <ellipse cxy="20 30" rxy="10 5" text="ellipse" style="font-size:1px" id="z"/>
  <line start="#a" end="#z"/>
</svg>"##;
    let expected = r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="55mm" height="60.5mm" viewBox="-12.5 -12.75 55 60.5">
  <style>
    <![CDATA[
    * { stroke-width: 0.2; fill: none; stroke: black; }
    ]]>
  </style>
  <rect x="0" y="0" width="5" height="5" id="a"/>
  <rect x="0" y="7" width="5" height="5"/>
  <rect x="0" y="14" width="5" height="5"/>
  <rect x="0" y="21" width="5" height="5"/>
  <ellipse cx="20" cy="30" rx="10" ry="5" style="font-size:1px" id="z"/>
  <text x="20" y="30" style="font-size:1px" class="tbox">ellipse</text>
  <line x1="5" y1="5" x2="10" y2="25"/>
</svg>"##;

    compare(input, expected);
}
