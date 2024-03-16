from random import randint, shuffle

ELEMENT_COUNT = 3000


def main():
    ids = [f'id{idx}' for idx in range(ELEMENT_COUNT)]
    shuffle(ids)

    dirspec = 'hHvV'

    lines = []
    with open('large-scale.xml', 'w') as f:
        f.write("<svg>\n")
        for idx, id in enumerate(ids):
            if idx == 0:
                f.write(f'  <rect id="{id}" xy="0" wh="1"/>\n')
            else:
                rel_id = ids[idx - 1]
                lines.append(f'  <rect id="{id}" xy="#{rel_id}:{dirspec[randint(0, 3)]} {randint(0, 3)}" wh="1"/>')
        shuffle(lines)
        f.write('\n'.join(lines))
        f.write("</svg>\n")



if __name__ == '__main__':
    main()
